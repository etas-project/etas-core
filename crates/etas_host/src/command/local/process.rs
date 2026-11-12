use std::{
    collections::{HashMap, VecDeque},
    future::pending,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command},
    sync::{mpsc, oneshot},
    time::{Instant, sleep_until},
};
use tokio_util::sync::CancellationToken;

use crate::{
    CommandProcessEvent, HostError, HostErrorCode, execution::CancelSignal,
    sandbox::platform::PreparedIsolation,
};

use super::{
    CommandExecutionPolicy,
    cleanup::{terminate_and_reap, terminate_uninitialized_child},
    process_tree::ProcessTreeController,
};

#[derive(Clone, Debug)]
pub(super) struct CommandProcessRegistry {
    processes: Arc<Mutex<HashMap<String, ProcessRecord>>>,
}

#[derive(Clone, Debug)]
struct ProcessRecord {
    program: String,
    stdin: Arc<tokio::sync::Mutex<Option<ChildStdin>>>,
    control: mpsc::Sender<ProcessControl>,
}

#[derive(Debug)]
enum ProcessControl {
    Read {
        max_bytes: usize,
        reply: oneshot::Sender<Result<CommandProcessEvent, HostError>>,
        cancellation: CancellationToken,
    },
    Stop {
        reply: oneshot::Sender<Result<CommandProcessEvent, HostError>>,
    },
}

struct ProcessWorker {
    child: Child,
    process_tree: ProcessTreeController,
    stdout: ChildStdout,
    stderr: ChildStderr,
    stdout_open: bool,
    stderr_open: bool,
    stdout_buffer: VecDeque<u8>,
    stderr_buffer: VecDeque<u8>,
    exit_code: Option<i32>,
    terminal: Option<ProcessTerminal>,
    pending_read: Option<PendingRead>,
    control: mpsc::Receiver<ProcessControl>,
    deadline: Option<Instant>,
    policy: CommandExecutionPolicy,
}

struct PendingRead {
    max_bytes: usize,
    reply: oneshot::Sender<Result<CommandProcessEvent, HostError>>,
    cancellation: CancellationToken,
}

struct CancelPendingReadOnDrop(CancellationToken);

impl Drop for CancelPendingReadOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

#[derive(Clone)]
enum ProcessTerminal {
    Exited(i32),
    Stopped,
    Failed(HostError),
}

impl std::fmt::Debug for ProcessWorker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProcessWorker")
            .field("stdout_buffer", &self.stdout_buffer.len())
            .field("stderr_buffer", &self.stderr_buffer.len())
            .field("exit_code", &self.exit_code)
            .finish_non_exhaustive()
    }
}

impl Default for CommandProcessRegistry {
    fn default() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl CommandProcessRegistry {
    pub(super) fn program(&self, id: &str) -> Result<String, HostError> {
        self.record(id).map(|record| record.program)
    }

    pub(super) async fn start(
        &self,
        mut command: Command,
        deadline: Option<Instant>,
        policy: CommandExecutionPolicy,
        program: String,
        isolation: PreparedIsolation,
    ) -> Result<(String, crate::CommandIsolationReport), HostError> {
        super::descriptors::configure(&mut command)?;
        let activation = isolation.install(&mut command);
        let mut child = command.spawn().map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to spawn background command",
            )
            .with_detail("program", program.clone())
            .with_detail("error", error.to_string())
        })?;
        let isolation = activation.after_spawn();
        let initialized = (|| {
            let process_tree = ProcessTreeController::for_child(&child, &program)?;
            let stdin = child.stdin.take().ok_or_else(|| missing_pipe("stdin"))?;
            let stdout = child.stdout.take().ok_or_else(|| missing_pipe("stdout"))?;
            let stderr = child.stderr.take().ok_or_else(|| missing_pipe("stderr"))?;
            Ok((process_tree, stdin, stdout, stderr))
        })();
        let (process_tree, stdin, stdout, stderr) = match initialized {
            Ok(parts) => parts,
            Err(error) => {
                terminate_uninitialized_child(&mut child, None).await?;
                return Err(error);
            }
        };
        let id = process_id()?;
        let (control, receiver) = mpsc::channel(16);
        let stdin = Arc::new(tokio::sync::Mutex::new(Some(stdin)));
        self.lock()?.insert(
            id.clone(),
            ProcessRecord {
                program,
                stdin: stdin.clone(),
                control,
            },
        );
        tokio::spawn(
            ProcessWorker {
                child,
                process_tree,
                stdout,
                stderr,
                stdout_open: true,
                stderr_open: true,
                stdout_buffer: VecDeque::new(),
                stderr_buffer: VecDeque::new(),
                exit_code: None,
                terminal: None,
                pending_read: None,
                control: receiver,
                deadline,
                policy,
            }
            .run(stdin),
        );
        Ok((id, isolation))
    }

    pub(super) async fn read(
        &self,
        id: &str,
        max_bytes: usize,
        policy: CommandExecutionPolicy,
        signal: Option<CancelSignal>,
    ) -> Result<CommandProcessEvent, HostError> {
        if max_bytes == 0 || max_bytes > policy.max_stdout_bytes.max(policy.max_stderr_bytes) {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "background command read limit is outside the configured bounds",
            )
            .with_detail("max_bytes", max_bytes.to_string()));
        }
        let record = self.record(id)?;
        let (reply, response) = oneshot::channel();
        let cancellation = CancellationToken::new();
        let _cancel_on_drop = CancelPendingReadOnDrop(cancellation.clone());
        record
            .control
            .send(ProcessControl::Read {
                max_bytes,
                reply,
                cancellation: cancellation.clone(),
            })
            .await
            .map_err(|_| process_unavailable(id))?;
        let mut response = response;
        let response = match signal {
            Some(signal) => {
                tokio::select! {
                    biased;
                    cancelled = signal.cancelled() => {
                        cancellation.cancel();
                        cancelled?;
                        (&mut response).await
                    }
                    response = &mut response => response,
                }
            }
            None => response.await,
        };
        response.map_err(|_| process_unavailable(id))?
    }

    pub(super) async fn write(&self, id: &str, bytes: Vec<u8>) -> Result<(), HostError> {
        let record = self.record(id)?;
        let mut stdin = record.stdin.lock().await;
        let Some(stdin) = stdin.as_mut() else {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "background command stdin is closed",
            )
            .with_detail("process", id.to_owned()));
        };
        stdin.write_all(&bytes).await.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to write background command stdin",
            )
            .with_detail("process", id.to_owned())
            .with_detail("error", error.to_string())
        })
    }

    pub(super) async fn close_stdin(&self, id: &str) -> Result<(), HostError> {
        let record = self.record(id)?;
        let mut stdin = record.stdin.lock().await;
        if let Some(mut open) = stdin.take() {
            open.shutdown().await.map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to close background command stdin",
                )
                .with_detail("process", id.to_owned())
                .with_detail("error", error.to_string())
            })?;
        }
        Ok(())
    }

    pub(super) async fn stop(&self, id: &str) -> Result<CommandProcessEvent, HostError> {
        let record = self.record(id)?;
        let (reply, response) = oneshot::channel();
        record
            .control
            .send(ProcessControl::Stop { reply })
            .await
            .map_err(|_| process_unavailable(id))?;
        response.await.map_err(|_| process_unavailable(id))?
    }

    fn record(&self, id: &str) -> Result<ProcessRecord, HostError> {
        self.lock()?
            .get(id)
            .cloned()
            .ok_or_else(|| process_unavailable(id))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, HashMap<String, ProcessRecord>>, HostError> {
        self.processes.lock().map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "background command registry is unavailable",
            )
        })
    }
}

impl ProcessWorker {
    async fn run(mut self, stdin: Arc<tokio::sync::Mutex<Option<ChildStdin>>>) {
        let mut stdout_chunk = [0_u8; 8192];
        let mut stderr_chunk = [0_u8; 8192];
        loop {
            self.finish_natural_exit();
            self.cancel_pending_read_if_requested();
            self.fulfill_pending_read();
            let read_cancellation = self
                .pending_read
                .as_ref()
                .map(|pending| pending.cancellation.clone());
            tokio::select! {
                _ = wait_for_read_cancellation(read_cancellation) => {
                    self.cancel_pending_read();
                }
                control = self.control.recv() => {
                    self.cancel_pending_read_if_requested();
                    match control {
                        Some(ProcessControl::Read { max_bytes, reply, cancellation }) => {
                            if self.pending_read.is_some() {
                                let _ = reply.send(Err(HostError::new(
                                    HostErrorCode::InvalidRequest,
                                    "background command already has a pending read",
                                )));
                            } else {
                                self.pending_read = Some(PendingRead {
                                    max_bytes,
                                    reply,
                                    cancellation,
                                });
                                self.fulfill_pending_read();
                            }
                        }
                        Some(ProcessControl::Stop { reply }) => {
                            let event = self.stop(stdin.clone()).await;
                            self.fulfill_pending_read();
                            let _ = reply.send(event);
                        }
                        None => {
                            if self.terminal.is_none() {
                                let _ = self.stop(stdin.clone()).await;
                            }
                            return;
                        }
                    }
                },
                read = self.stdout.read(&mut stdout_chunk), if self.stdout_open && self.terminal.is_none() => {
                    match read {
                        Ok(0) => self.stdout_open = false,
                        Ok(read) => self.append_stdout(&stdout_chunk[..read]).await,
                        Err(error) => self.fail(output_read_error("stdout", error), stdin.clone()).await,
                    }
                }
                read = self.stderr.read(&mut stderr_chunk), if self.stderr_open && self.terminal.is_none() => {
                    match read {
                        Ok(0) => self.stderr_open = false,
                        Ok(read) => self.append_stderr(&stderr_chunk[..read]).await,
                        Err(error) => self.fail(output_read_error("stderr", error), stdin.clone()).await,
                    }
                }
                status = self.child.wait(), if self.exit_code.is_none() && self.terminal.is_none() => {
                    match status {
                        Ok(status) => self.exit_code = Some(status.code().unwrap_or(-1)),
                        Err(error) => self.fail(HostError::new(
                            HostErrorCode::ProviderUnavailable,
                            "failed to wait for background command",
                        ).with_detail("error", error.to_string()), stdin.clone()).await,
                    }
                }
                _ = wait_for_deadline(self.deadline), if self.terminal.is_none() => {
                    self.fail(HostError::new(
                        HostErrorCode::BudgetExceeded,
                        "background command exceeded the run-owned time budget",
                    ), stdin.clone()).await;
                }
            }
        }
    }

    fn finish_natural_exit(&mut self) {
        if self.terminal.is_none()
            && !self.stdout_open
            && !self.stderr_open
            && let Some(exit_code) = self.exit_code
        {
            self.terminal = Some(ProcessTerminal::Exited(exit_code));
        }
    }

    fn fulfill_pending_read(&mut self) {
        let Some(pending) = self.pending_read.take() else {
            return;
        };
        if pending.cancellation.is_cancelled() {
            let _ = pending.reply.send(Err(cancelled_read()));
            return;
        }
        if pending.reply.is_closed() {
            return;
        }
        if self.stdout_buffer.is_empty() && self.stderr_buffer.is_empty() && self.terminal.is_none()
        {
            self.pending_read = Some(pending);
            return;
        }
        let stdout = drain(&mut self.stdout_buffer, pending.max_bytes);
        let stderr = drain(&mut self.stderr_buffer, pending.max_bytes);
        let result = match &self.terminal {
            Some(ProcessTerminal::Failed(error)) => Err(error.clone()),
            Some(ProcessTerminal::Exited(exit_code))
                if self.stdout_buffer.is_empty() && self.stderr_buffer.is_empty() =>
            {
                Ok(CommandProcessEvent::Exited {
                    exit_code: *exit_code,
                    stdout,
                    stderr,
                })
            }
            Some(ProcessTerminal::Stopped)
                if self.stdout_buffer.is_empty() && self.stderr_buffer.is_empty() =>
            {
                Ok(CommandProcessEvent::Stopped { stdout, stderr })
            }
            _ => Ok(CommandProcessEvent::Output { stdout, stderr }),
        };
        let _ = pending.reply.send(result);
    }

    fn cancel_pending_read_if_requested(&mut self) {
        if self
            .pending_read
            .as_ref()
            .is_some_and(|pending| pending.cancellation.is_cancelled())
        {
            self.cancel_pending_read();
        }
    }

    fn cancel_pending_read(&mut self) {
        if let Some(pending) = self.pending_read.take() {
            let _ = pending.reply.send(Err(cancelled_read()));
        }
    }

    async fn append_stdout(&mut self, bytes: &[u8]) {
        if append_bounded(
            &mut self.stdout_buffer,
            bytes,
            "stdout",
            self.policy.max_stdout_bytes,
        )
        .is_err()
        {
            let error = output_limit_exceeded("stdout", self.policy.max_stdout_bytes);
            self.fail_without_stdin(error).await;
        }
    }

    async fn append_stderr(&mut self, bytes: &[u8]) {
        if append_bounded(
            &mut self.stderr_buffer,
            bytes,
            "stderr",
            self.policy.max_stderr_bytes,
        )
        .is_err()
        {
            let error = output_limit_exceeded("stderr", self.policy.max_stderr_bytes);
            self.fail_without_stdin(error).await;
        }
    }

    async fn stop(
        &mut self,
        stdin: Arc<tokio::sync::Mutex<Option<ChildStdin>>>,
    ) -> Result<CommandProcessEvent, HostError> {
        if let Some(terminal) = &self.terminal {
            return self.take_terminal_event(terminal.clone());
        }
        stdin.lock().await.take();
        terminate_and_reap(&mut self.child, self.process_tree, None).await?;
        self.drain_after_termination().await?;
        self.terminal = Some(ProcessTerminal::Stopped);
        self.take_terminal_event(ProcessTerminal::Stopped)
    }

    async fn fail(&mut self, error: HostError, stdin: Arc<tokio::sync::Mutex<Option<ChildStdin>>>) {
        stdin.lock().await.take();
        self.fail_without_stdin(error).await;
    }

    async fn fail_without_stdin(&mut self, error: HostError) {
        let cleanup = terminate_and_reap(&mut self.child, self.process_tree, None).await;
        self.terminal = Some(ProcessTerminal::Failed(cleanup.err().unwrap_or(error)));
        self.stdout_open = false;
        self.stderr_open = false;
    }

    async fn drain_after_termination(&mut self) -> Result<(), HostError> {
        let stdout_limit = self
            .policy
            .max_stdout_bytes
            .saturating_sub(self.stdout_buffer.len());
        let stderr_limit = self
            .policy
            .max_stderr_bytes
            .saturating_sub(self.stderr_buffer.len());
        let (stdout, stderr) = tokio::join!(
            read_remaining(&mut self.stdout, "stdout", stdout_limit),
            read_remaining(&mut self.stderr, "stderr", stderr_limit),
        );
        self.stdout_buffer.extend(stdout?);
        self.stderr_buffer.extend(stderr?);
        self.stdout_open = false;
        self.stderr_open = false;
        Ok(())
    }

    fn take_terminal_event(
        &mut self,
        terminal: ProcessTerminal,
    ) -> Result<CommandProcessEvent, HostError> {
        let stdout = self.stdout_buffer.drain(..).collect();
        let stderr = self.stderr_buffer.drain(..).collect();
        match terminal {
            ProcessTerminal::Exited(exit_code) => Ok(CommandProcessEvent::Exited {
                exit_code,
                stdout,
                stderr,
            }),
            ProcessTerminal::Stopped => Ok(CommandProcessEvent::Stopped { stdout, stderr }),
            ProcessTerminal::Failed(error) => Err(error),
        }
    }
}

async fn wait_for_read_cancellation(cancellation: Option<CancellationToken>) {
    match cancellation {
        Some(cancellation) => cancellation.cancelled().await,
        None => pending::<()>().await,
    }
}

fn cancelled_read() -> HostError {
    HostError::new(
        HostErrorCode::Cancelled,
        "background command read was cancelled",
    )
}

fn process_id() -> Result<String, HostError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to create a background command identifier",
        )
        .with_detail("error", error.to_string())
    })?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn append_bounded(
    output: &mut VecDeque<u8>,
    bytes: &[u8],
    stream: &'static str,
    limit: usize,
) -> Result<(), HostError> {
    if output
        .len()
        .checked_add(bytes.len())
        .is_none_or(|len| len > limit)
    {
        return Err(output_limit_exceeded(stream, limit));
    }
    output.extend(bytes);
    Ok(())
}

fn drain(output: &mut VecDeque<u8>, max_bytes: usize) -> Vec<u8> {
    let count = output.len().min(max_bytes);
    output.drain(..count).collect()
}

async fn read_remaining(
    reader: &mut (impl tokio::io::AsyncRead + Unpin),
    stream: &'static str,
    limit: usize,
) -> Result<Vec<u8>, HostError> {
    let mut output = Vec::new();
    reader
        .read_to_end(&mut output)
        .await
        .map_err(|error| output_read_error(stream, error))?;
    if output.len() > limit {
        return Err(output_limit_exceeded(stream, limit));
    }
    Ok(output)
}

fn output_read_error(stream: &'static str, error: std::io::Error) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "failed to read background command output",
    )
    .with_detail("stream", stream)
    .with_detail("error", error.to_string())
}

fn output_limit_exceeded(stream: &'static str, limit: usize) -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "background command output exceeded the configured unread limit",
    )
    .with_detail("stream", stream)
    .with_detail("limit_bytes", limit.to_string())
}

fn process_unavailable(id: &str) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "background command is not available for reconnection",
    )
    .with_detail("process", id.to_owned())
}

fn missing_pipe(stream: &'static str) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "spawned background command is missing a configured pipe",
    )
    .with_detail("stream", stream)
}

async fn wait_for_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => pending().await,
    }
}
