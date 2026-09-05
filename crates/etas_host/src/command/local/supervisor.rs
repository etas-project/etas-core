use std::{
    future::pending,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, Command},
    sync::oneshot,
    task::JoinHandle,
    time::{Instant, sleep_until},
};

use crate::{CommandOutput, HostError, HostErrorCode};

use super::{CommandExecutionPolicy, output::collect_bounded, process_tree::ProcessTreeController};

pub(super) struct SupervisedCommand {
    cancellation: Option<oneshot::Sender<()>>,
    process_tree: ProcessTreeController,
    completed: Arc<AtomicBool>,
    task: JoinHandle<Result<CommandOutput, HostError>>,
}

struct CommandSupervisor {
    child: Child,
    process_tree: ProcessTreeController,
    stdin: Option<ChildStdin>,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    input: Option<Vec<u8>>,
    cancellation: oneshot::Receiver<()>,
    deadline: Option<Instant>,
    policy: CommandExecutionPolicy,
}

impl SupervisedCommand {
    pub(super) fn spawn(
        mut command: Command,
        input: Option<Vec<u8>>,
        deadline: Option<Instant>,
        policy: CommandExecutionPolicy,
        program: String,
    ) -> Result<Self, HostError> {
        let mut child = command.spawn().map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to spawn command",
            )
            .with_detail("program", program.clone())
            .with_detail("error", error.to_string())
        })?;
        let process_tree = ProcessTreeController::for_child(&child, &program)?;
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().ok_or_else(|| missing_pipe("stdout"))?;
        let stderr = child.stderr.take().ok_or_else(|| missing_pipe("stderr"))?;
        let (cancel, cancellation) = oneshot::channel();
        let completed = Arc::new(AtomicBool::new(false));
        let task_completed = Arc::clone(&completed);
        let supervisor = CommandSupervisor {
            child,
            process_tree,
            stdin,
            stdout,
            stderr,
            input,
            cancellation,
            deadline,
            policy,
        };
        let task = tokio::spawn(async move {
            let result = supervisor.run().await;
            task_completed.store(true, Ordering::Release);
            result
        });
        Ok(Self {
            cancellation: Some(cancel),
            process_tree,
            completed,
            task,
        })
    }

    pub(super) async fn wait(mut self) -> Result<CommandOutput, HostError> {
        let result = (&mut self.task).await.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "command supervisor task failed",
            )
            .with_detail("error", error.to_string())
        })?;
        self.cancellation = None;
        result
    }
}

impl Drop for SupervisedCommand {
    fn drop(&mut self) {
        if self.completed.load(Ordering::Acquire) {
            return;
        }
        if let Some(cancellation) = self.cancellation.take() {
            let _ = cancellation.send(());
        }
        self.process_tree.kill_from_drop();
    }
}

impl CommandSupervisor {
    async fn run(self) -> Result<CommandOutput, HostError> {
        let Self {
            mut child,
            process_tree,
            stdin,
            stdout,
            stderr,
            input,
            mut cancellation,
            deadline,
            policy,
        } = self;
        let mut stdin_future = Box::pin(write_stdin(stdin, input));
        let mut stdout_future =
            Box::pin(collect_bounded(stdout, "stdout", policy.max_stdout_bytes));
        let mut stderr_future =
            Box::pin(collect_bounded(stderr, "stderr", policy.max_stderr_bytes));
        let mut deadline_future = Box::pin(wait_for_deadline(deadline));
        let mut stdin_done = false;
        let mut stdout_result: Option<Vec<u8>> = None;
        let mut stderr_result: Option<Vec<u8>> = None;
        let mut status: Option<std::process::ExitStatus> = None;

        loop {
            if stdin_done && status.is_some() && stdout_result.is_some() && stderr_result.is_some()
            {
                let (Some(status), Some(stdout), Some(stderr)) =
                    (status.take(), stdout_result.take(), stderr_result.take())
                else {
                    return Err(HostError::new(
                        HostErrorCode::ProviderUnavailable,
                        "command supervisor completion state is inconsistent",
                    ));
                };
                return Ok(CommandOutput {
                    exit_code: status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                });
            }

            tokio::select! {
            _ = &mut cancellation => {
                terminate_and_reap(&mut child, process_tree).await?;
                return Err(HostError::new(
                    HostErrorCode::Cancelled,
                    "command execution was cancelled",
                ));
            }
            _ = &mut deadline_future => {
                terminate_and_reap(&mut child, process_tree).await?;
                return Err(HostError::new(
                    HostErrorCode::BudgetExceeded,
                    "command exceeded the run-owned time budget",
                ));
            }
            result = &mut stdin_future, if !stdin_done => {
                if let Err(error) = result {
                    terminate_and_reap(&mut child, process_tree).await?;
                    return Err(error);
                }
                stdin_done = true;
            }
            result = &mut stdout_future, if stdout_result.is_none() => {
                match result {
                    Ok(output) => stdout_result = Some(output),
                    Err(error) => {
                        terminate_and_reap(&mut child, process_tree).await?;
                        return Err(error);
                    }
                }
            }
            result = &mut stderr_future, if stderr_result.is_none() => {
                match result {
                    Ok(output) => stderr_result = Some(output),
                    Err(error) => {
                        terminate_and_reap(&mut child, process_tree).await?;
                        return Err(error);
                    }
                }
            }
            result = child.wait(), if status.is_none() => {
                status = Some(result.map_err(|error| {
                    HostError::new(
                        HostErrorCode::ProviderUnavailable,
                        "failed to wait for command",
                    )
                    .with_detail("error", error.to_string())
                })?);
            }
            }
        }
    }
}

async fn write_stdin(
    mut stdin: Option<ChildStdin>,
    input: Option<Vec<u8>>,
) -> Result<(), HostError> {
    let Some(mut stdin) = stdin.take() else {
        return Ok(());
    };
    if let Some(input) = input {
        stdin.write_all(&input).await.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to write command stdin",
            )
            .with_detail("error", error.to_string())
        })?;
    }
    stdin.shutdown().await.map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to close command stdin",
        )
        .with_detail("error", error.to_string())
    })
}

async fn wait_for_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => pending().await,
    }
}

async fn terminate_and_reap(
    child: &mut Child,
    process_tree: ProcessTreeController,
) -> Result<(), HostError> {
    if child.try_wait().map_err(wait_error)?.is_some() {
        return Ok(());
    }
    if let Err(control_error) = process_tree.kill(child)
        && child.try_wait().map_err(wait_error)?.is_none()
    {
        return Err(control_error);
    }
    wait_result(child.wait().await)?;
    Ok(())
}

fn wait_result(
    result: Result<std::process::ExitStatus, std::io::Error>,
) -> Result<std::process::ExitStatus, HostError> {
    result.map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to reap terminated command",
        )
        .with_detail("error", error.to_string())
    })
}

fn wait_error(error: std::io::Error) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "failed to query command process state",
    )
    .with_detail("error", error.to_string())
}

fn missing_pipe(stream: &'static str) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "spawned command is missing a configured output pipe",
    )
    .with_detail("stream", stream)
}
