use std::{
    future::Future,
    io::{BufRead, Write},
    pin::Pin,
    sync::{Arc, OnceLock},
};

use tokio::sync::{Mutex, mpsc};

use crate::{ActionInstance, ExecutionBudget, HostError, HostErrorCode, HostRequestId};

use super::{ConsoleClient, ConsoleOperation, ConsoleRequest, ConsoleResponse, ConsoleResult};

#[derive(Clone, Debug)]
pub struct LocalStdioClient {
    input: Arc<ConsoleInputBroker>,
}

#[derive(Debug)]
struct ConsoleInputBroker {
    state: Mutex<ConsoleInputState>,
}

#[derive(Debug)]
struct ConsoleInputState {
    receiver: mpsc::UnboundedReceiver<ConsoleInputEvent>,
    eof: bool,
}

#[derive(Debug)]
enum ConsoleInputEvent {
    Line(String),
    Eof,
    Failed(String),
}

static PROCESS_STDIN: OnceLock<Arc<ConsoleInputBroker>> = OnceLock::new();

impl LocalStdioClient {
    pub fn new() -> Self {
        Self {
            input: PROCESS_STDIN
                .get_or_init(|| Arc::new(ConsoleInputBroker::system_stdin()))
                .clone(),
        }
    }

    #[cfg(test)]
    fn with_input(receiver: mpsc::UnboundedReceiver<ConsoleInputEvent>) -> Self {
        Self {
            input: Arc::new(ConsoleInputBroker::new(receiver)),
        }
    }

    fn require_console_authority(request: &ConsoleRequest) -> Result<(), HostError> {
        let action = console_action(&request.operation);
        if request.authority.allows(&action) {
            return Ok(());
        }
        Err(HostError::new(
            HostErrorCode::AuthorityDenied,
            "console host request is missing required Console action grant",
        )
        .with_detail("action", format!("{}.{}", action.effect, action.action))
        .with_detail("request_id", request.id.0.to_string()))
    }

    async fn execute_local(&self, request: ConsoleRequest) -> Result<ConsoleResponse, HostError> {
        Self::require_console_authority(&request)?;
        request.budget.check_time()?;
        let result = match request.operation {
            ConsoleOperation::ReadAllStdin => {
                ConsoleResult::Input(self.input.read_all(&request.budget, request.id).await?)
            }
            ConsoleOperation::ReadLineStdin => {
                ConsoleResult::Input(self.input.read_line(&request.budget, request.id).await?)
            }
            ConsoleOperation::WriteStdout { text, newline } => {
                write_output(OutputStream::Stdout, text, newline)?;
                ConsoleResult::Written
            }
            ConsoleOperation::WriteStderr { text, newline } => {
                write_output(OutputStream::Stderr, text, newline)?;
                ConsoleResult::Written
            }
        };
        request.budget.check_time()?;
        Ok(ConsoleResponse {
            id: request.id,
            result,
        })
    }
}

impl Default for LocalStdioClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleInputBroker {
    fn system_stdin() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let input_sender = sender.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("etas-console-stdin".into())
            .spawn(move || {
                let stdin = std::io::stdin();
                let mut input = stdin.lock();
                loop {
                    let mut line = String::new();
                    match input.read_line(&mut line) {
                        Ok(0) => {
                            let _ = input_sender.send(ConsoleInputEvent::Eof);
                            break;
                        }
                        Ok(_) => {
                            if input_sender.send(ConsoleInputEvent::Line(line)).is_err() {
                                break;
                            }
                        }
                        Err(error) => {
                            let _ = input_sender.send(ConsoleInputEvent::Failed(error.to_string()));
                            break;
                        }
                    }
                }
            })
        {
            let _ = sender.send(ConsoleInputEvent::Failed(format!(
                "failed to start stdin broker: {error}"
            )));
        }
        Self::new(receiver)
    }

    fn new(receiver: mpsc::UnboundedReceiver<ConsoleInputEvent>) -> Self {
        Self {
            state: Mutex::new(ConsoleInputState {
                receiver,
                eof: false,
            }),
        }
    }

    async fn read_line(
        &self,
        budget: &ExecutionBudget,
        request_id: HostRequestId,
    ) -> Result<String, HostError> {
        match self.next_event(budget, request_id).await? {
            ConsoleInputEvent::Line(line) => Ok(line),
            ConsoleInputEvent::Eof => Ok(String::new()),
            ConsoleInputEvent::Failed(error) => Err(stdin_error(error)),
        }
    }

    async fn read_all(
        &self,
        budget: &ExecutionBudget,
        request_id: HostRequestId,
    ) -> Result<String, HostError> {
        let mut input = String::new();
        loop {
            match self.next_event(budget, request_id).await? {
                ConsoleInputEvent::Line(line) => input.push_str(&line),
                ConsoleInputEvent::Eof => return Ok(input),
                ConsoleInputEvent::Failed(error) => return Err(stdin_error(error)),
            }
        }
    }

    async fn next_event(
        &self,
        budget: &ExecutionBudget,
        request_id: HostRequestId,
    ) -> Result<ConsoleInputEvent, HostError> {
        budget.check_time()?;
        let deadline = budget.deadline()?;
        let mut state = self.state.lock().await;
        if state.eof {
            return Ok(ConsoleInputEvent::Eof);
        }
        let event = match deadline {
            Some(deadline) => tokio::select! {
                event = state.receiver.recv() => event,
                _ = tokio::time::sleep_until(deadline) => {
                    return Err(console_deadline_exceeded(request_id));
                }
            },
            None => state.receiver.recv().await,
        }
        .ok_or_else(|| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "console stdin broker stopped before reaching end of input",
            )
        })?;
        if matches!(event, ConsoleInputEvent::Eof) {
            state.eof = true;
        }
        Ok(event)
    }
}

#[derive(Clone, Copy)]
enum OutputStream {
    Stdout,
    Stderr,
}

fn write_output(stream: OutputStream, text: String, newline: bool) -> Result<(), HostError> {
    let result = match stream {
        OutputStream::Stdout => {
            let mut output = std::io::stdout().lock();
            write_text(&mut output, &text, newline)
        }
        OutputStream::Stderr => {
            let mut output = std::io::stderr().lock();
            write_text(&mut output, &text, newline)
        }
    };
    result.map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to write console output",
        )
        .with_detail("error", error.to_string())
    })
}

fn write_text(output: &mut impl Write, text: &str, newline: bool) -> std::io::Result<()> {
    if newline {
        writeln!(output, "{text}")?;
    } else {
        write!(output, "{text}")?;
    }
    output.flush()
}

fn stdin_error(error: String) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, "failed to read stdin")
        .with_detail("error", error)
}

fn console_deadline_exceeded(request_id: HostRequestId) -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "console input exceeded the run-owned time budget",
    )
    .with_detail("request_id", request_id.0.to_string())
}

fn console_action(operation: &ConsoleOperation) -> ActionInstance {
    let action = match operation {
        ConsoleOperation::ReadAllStdin => "stdin_read_all",
        ConsoleOperation::ReadLineStdin => "stdin_read_line",
        ConsoleOperation::WriteStdout { .. } => "stdout_write",
        ConsoleOperation::WriteStderr { .. } => "stderr_write",
    };
    ActionInstance::new("Console", action, Vec::new())
}

impl ConsoleClient for LocalStdioClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<ConsoleResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: ConsoleRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move { self.execute_local(request).await })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        AuthorityContext, Budget, HostActionGrant, SandboxPolicy, TimeBudget, TraceContext, TraceId,
    };

    use super::*;

    fn read_request(id: u32, budget: ExecutionBudget) -> ConsoleRequest {
        ConsoleRequest {
            id: HostRequestId(id),
            operation: ConsoleOperation::ReadLineStdin,
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Console", "stdin_read_line")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(1)),
            budget,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_line_respects_run_owned_deadline_without_blocking_executor() {
        let (_sender, receiver) = mpsc::unbounded_channel();
        let client = LocalStdioClient::with_input(receiver);
        let budget = ExecutionBudget::start(Budget {
            time: Some(TimeBudget { max_millis: 25 }),
            ..Budget::default()
        });

        let error = client
            .execute(read_request(1, budget))
            .await
            .expect_err("missing input must time out");

        assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_read_does_not_consume_the_next_line() {
        let (sender, receiver) = mpsc::unbounded_channel();
        let client = LocalStdioClient::with_input(receiver);
        let pending = tokio::spawn({
            let client = client.clone();
            async move {
                client
                    .execute(read_request(1, ExecutionBudget::default()))
                    .await
            }
        });
        tokio::task::yield_now().await;
        pending.abort();
        let _ = pending.await;

        sender
            .send(ConsoleInputEvent::Line("next\n".into()))
            .expect("test input receiver");
        let response = tokio::time::timeout(
            Duration::from_secs(1),
            client.execute(read_request(2, ExecutionBudget::default())),
        )
        .await
        .expect("second read must not block")
        .expect("second read must succeed");

        assert_eq!(response.result, ConsoleResult::Input("next\n".into()));
    }
}
