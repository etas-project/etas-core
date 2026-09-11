use std::future::pending;

use tokio::{
    io::AsyncWriteExt,
    process::{Child, ChildStdin, Command},
    sync::oneshot,
    task::JoinHandle,
    time::{Instant, sleep_until},
};

use crate::{CommandOutput, HostError, HostErrorCode};

use super::cleanup::{terminate_and_reap, terminate_uninitialized_child};
use super::{CommandExecutionPolicy, output::collect_bounded, process_tree::ProcessTreeController};

pub(super) struct SupervisedCommand {
    cancellation: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<CommandOutput, HostError>>,
}

struct CommandSupervisor {
    operation: Option<crate::execution::OperationContext>,
    isolation: crate::CommandIsolationReport,
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

struct SpawnedCommand {
    child: Child,
    operation: Option<crate::execution::OperationContext>,
    isolation: crate::CommandIsolationReport,
    input: Option<Vec<u8>>,
    cancellation: oneshot::Receiver<()>,
    deadline: Option<Instant>,
    policy: CommandExecutionPolicy,
    program: String,
}

impl SpawnedCommand {
    async fn run(mut self) -> Result<CommandOutput, HostError> {
        let initialized = (|| {
            let process_tree = ProcessTreeController::for_child(&self.child, &self.program)?;
            let stdout = self
                .child
                .stdout
                .take()
                .ok_or_else(|| missing_pipe("stdout"))?;
            let stderr = self
                .child
                .stderr
                .take()
                .ok_or_else(|| missing_pipe("stderr"))?;
            Ok((process_tree, stdout, stderr))
        })();
        let (process_tree, stdout, stderr) = match initialized {
            Ok(parts) => parts,
            Err(error) => {
                terminate_uninitialized_child(&mut self.child, self.operation.as_ref()).await?;
                return Err(error);
            }
        };
        let stdin = self.child.stdin.take();
        CommandSupervisor {
            child: self.child,
            operation: self.operation,
            isolation: self.isolation,
            process_tree,
            stdin,
            stdout,
            stderr,
            input: self.input,
            cancellation: self.cancellation,
            deadline: self.deadline,
            policy: self.policy,
        }
        .run()
        .await
    }
}

impl SupervisedCommand {
    pub(super) fn spawn(
        mut command: Command,
        input: Option<Vec<u8>>,
        deadline: Option<Instant>,
        policy: CommandExecutionPolicy,
        program: String,
        isolation: crate::sandbox::platform::PreparedIsolation,
        operation: Option<crate::execution::OperationContext>,
    ) -> Result<Self, HostError> {
        super::descriptors::configure(&mut command)?;
        let activation = isolation.install(&mut command);
        let child = command.spawn().map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to spawn command",
            )
            .with_detail("program", program.clone())
            .with_detail("error", error.to_string())
        })?;
        let isolation = activation.after_spawn();
        let (cancel, cancellation) = oneshot::channel();
        let supervisor = SpawnedCommand {
            operation,
            isolation,
            child,
            input,
            cancellation,
            deadline,
            policy,
            program,
        };
        let task = tokio::spawn(supervisor.run());
        Ok(Self {
            cancellation: Some(cancel),
            task,
        })
    }

    pub(super) async fn wait(
        mut self,
        signal: Option<crate::execution::CancelSignal>,
    ) -> Result<CommandOutput, HostError> {
        let result = tokio::select! {
            result = &mut self.task => result,
            _ = async { match signal { Some(signal) => { let _ = signal.cancelled().await; }, None => pending::<()>().await } } => {
                if let Some(cancellation) = self.cancellation.take() { let _ = cancellation.send(()); }
                // The supervisor retains process ownership through kill and reap.
                (&mut self.task).await
            }
        }.map_err(|error| {
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
        if let Some(cancellation) = self.cancellation.take() {
            let _ = cancellation.send(());
        }
    }
}

impl CommandSupervisor {
    async fn run(self) -> Result<CommandOutput, HostError> {
        let Self {
            operation,
            isolation,
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
                    isolation,
                    exit_code: status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                });
            }

            tokio::select! {
            _ = &mut cancellation => {
                terminate_and_reap(&mut child, process_tree, operation.as_ref()).await?;
                return Err(HostError::new(
                    HostErrorCode::Cancelled,
                    "command execution was cancelled",
                ));
            }
            _ = &mut deadline_future => {
                terminate_and_reap(&mut child, process_tree, operation.as_ref()).await?;
                return Err(HostError::new(
                    HostErrorCode::BudgetExceeded,
                    "command exceeded the run-owned time budget",
                ));
            }
            result = &mut stdin_future, if !stdin_done => {
                if let Err(error) = result {
                    terminate_and_reap(&mut child, process_tree, operation.as_ref()).await?;
                    return Err(error);
                }
                stdin_done = true;
            }
            result = &mut stdout_future, if stdout_result.is_none() => {
                match result {
                    Ok(output) => stdout_result = Some(output),
                    Err(error) => {
                        terminate_and_reap(&mut child, process_tree, operation.as_ref()).await?;
                        return Err(error);
                    }
                }
            }
            result = &mut stderr_future, if stderr_result.is_none() => {
                match result {
                    Ok(output) => stderr_result = Some(output),
                    Err(error) => {
                        terminate_and_reap(&mut child, process_tree, operation.as_ref()).await?;
                        return Err(error);
                    }
                }
            }
            result = child.wait(), if status.is_none() => {
                match result {
                    Ok(result) => status = Some(result),
                    Err(error) => {
                        let error = HostError::new(HostErrorCode::ProviderUnavailable, "failed to wait for command")
                            .with_detail("error", error.to_string());
                        terminate_and_reap(&mut child, process_tree, operation.as_ref()).await?;
                        return Err(error);
                    }
                }
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

fn missing_pipe(stream: &'static str) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "spawned command is missing a configured output pipe",
    )
    .with_detail("stream", stream)
}

#[cfg(test)]
mod tests;
