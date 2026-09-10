use std::{
    future::Future,
    io::Write,
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
    buffered: String,
    receiver: mpsc::Receiver<ConsoleInputEvent>,
    requests: Option<mpsc::Sender<()>>,
    pending: bool,
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
    fn with_input(receiver: mpsc::Receiver<ConsoleInputEvent>) -> Self {
        Self {
            input: Arc::new(ConsoleInputBroker::new(receiver, None)),
        }
    }

    /// Read host UI input through the same stdin owner as Console requests.
    /// This does not grant Console authority to an Etas program.
    pub async fn read_prompt_line(&self) -> Result<String, HostError> {
        self.input.read_serialized(false).await
    }

    pub async fn read_prompt_line_scoped(
        &self,
        operation: &crate::execution::OperationContext,
    ) -> Result<String, HostError> {
        operation.signal().check()?;
        tokio::select! {
            _ = operation.signal().cancelled() => Err(HostError::new(HostErrorCode::Cancelled, "approval input cancelled")),
            result = self.input.read_serialized(false) => result,
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

    async fn execute_local(
        &self,
        request: ConsoleRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<ConsoleResponse, HostError> {
        if let Some(operation) = operation {
            operation.signal().check()?;
        }
        Self::require_console_authority(&request)?;
        request.budget.check_time()?;
        let result = match request.operation {
            ConsoleOperation::ReadAllStdin => ConsoleResult::Input(
                self.input
                    .read(&request.budget, request.id, true, operation)
                    .await?,
            ),
            ConsoleOperation::ReadLineStdin => ConsoleResult::Input(
                self.input
                    .read(&request.budget, request.id, false, operation)
                    .await?,
            ),
            ConsoleOperation::WriteStdout { text, newline } => {
                write_output_async(OutputStream::Stdout, text, newline).await?;
                ConsoleResult::Written
            }
            ConsoleOperation::WriteStderr { text, newline } => {
                write_output_async(OutputStream::Stderr, text, newline).await?;
                ConsoleResult::Written
            }
        };
        request.budget.check_time()?;
        Ok(ConsoleResponse {
            id: request.id,
            result,
        })
    }

    pub async fn execute_scoped(
        &self,
        request: ConsoleRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<ConsoleResponse, HostError> {
        let client = self.clone();
        operation
            .supervise(
                move |context| async move { client.execute_local(request, Some(&context)).await },
            )
            .await
    }
}

impl Default for LocalStdioClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleInputBroker {
    fn system_stdin() -> Self {
        Self::with_reader(|line| std::io::stdin().read_line(line))
    }

    fn with_reader(
        mut read_line: impl FnMut(&mut String) -> std::io::Result<usize> + Send + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(1);
        let (requests, mut demand) = mpsc::channel(1);
        let input_sender = sender.clone();
        if let Err(error) = std::thread::Builder::new()
            .name("etas-console-stdin".into())
            .spawn(move || {
                while demand.blocking_recv().is_some() {
                    let mut line = String::new();
                    match read_line(&mut line) {
                        Ok(0) => {
                            let _ = input_sender.blocking_send(ConsoleInputEvent::Eof);
                            break;
                        }
                        Ok(_) => {
                            if input_sender
                                .blocking_send(ConsoleInputEvent::Line(line))
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(error) => {
                            let _ = input_sender
                                .blocking_send(ConsoleInputEvent::Failed(error.to_string()));
                            break;
                        }
                    }
                }
            })
        {
            let _ = sender.try_send(ConsoleInputEvent::Failed(format!(
                "failed to start stdin broker: {error}"
            )));
            return Self::new(receiver, None);
        }
        Self::new(receiver, Some(requests))
    }

    fn new(
        receiver: mpsc::Receiver<ConsoleInputEvent>,
        requests: Option<mpsc::Sender<()>>,
    ) -> Self {
        Self {
            state: Mutex::new(ConsoleInputState {
                buffered: String::new(),
                receiver,
                requests,
                pending: false,
                eof: false,
            }),
        }
    }

    async fn read(
        &self,
        budget: &ExecutionBudget,
        request_id: HostRequestId,
        all: bool,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<String, HostError> {
        budget.check_time()?;
        let deadline = budget.deadline()?;
        let read = async {
            match operation {
                Some(operation) => tokio::select! {
                    _ = operation.signal().cancelled() => Err(HostError::new(HostErrorCode::Cancelled, "console input cancelled")),
                    result = self.read_serialized(all) => result,
                },
                None => self.read_serialized(all).await,
            }
        };
        match deadline {
            Some(deadline) => tokio::select! {
                biased;
                _ = tokio::time::sleep_until(deadline) => Err(console_deadline_exceeded(request_id)),
                result = read => result,
            },
            None => read.await,
        }
    }

    async fn read_serialized(&self, all: bool) -> Result<String, HostError> {
        // One operation owns input; dropping its future releases the async lock.
        let mut state = self.state.lock().await;
        loop {
            if !all && let Some(end) = state.buffered.find('\n') {
                return Ok(state.buffered.drain(..=end).collect());
            }
            match state.next_event().await? {
                ConsoleInputEvent::Line(line) => {
                    state.buffered.push_str(&line);
                    if !all {
                        return Ok(std::mem::take(&mut state.buffered));
                    }
                }
                ConsoleInputEvent::Eof => return Ok(std::mem::take(&mut state.buffered)),
                ConsoleInputEvent::Failed(error) => return Err(stdin_error(error)),
            }
        }
    }
}

impl ConsoleInputState {
    async fn next_event(&mut self) -> Result<ConsoleInputEvent, HostError> {
        if self.eof {
            return Ok(ConsoleInputEvent::Eof);
        }
        if !self.pending {
            if let Some(requests) = &self.requests {
                requests
                    .try_send(())
                    .map_err(|error| stdin_error(error.to_string()))?;
            }
            // Retain the in-flight read across cancellation of its async consumer.
            self.pending = true;
        }
        let event = self.receiver.recv().await.ok_or_else(|| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "console stdin broker stopped before reaching end of input",
            )
        })?;
        self.pending = false;
        if matches!(event, ConsoleInputEvent::Eof) {
            self.eof = true;
        }
        Ok(event)
    }
}

#[derive(Clone, Copy)]
enum OutputStream {
    Stdout,
    Stderr,
}

async fn write_output_async(
    stream: OutputStream,
    text: String,
    newline: bool,
) -> Result<(), HostError> {
    tokio::task::spawn_blocking(move || write_output(stream, text, newline))
        .await
        .map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "console output worker failed",
            )
            .with_detail("error", error.to_string())
        })?
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
        Box::pin(async move { self.execute_local(request, None).await })
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
        let (_sender, receiver) = mpsc::channel(1);
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
    async fn broker_reads_only_on_demand_and_shares_input_with_host_prompts() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let reads = Arc::new(AtomicUsize::new(0));
        let calls = Arc::clone(&reads);
        let (started, reader_started) = std::sync::mpsc::channel();
        let broker = ConsoleInputBroker::with_reader(move |line| {
            let index = calls.fetch_add(1, Ordering::SeqCst);
            started.send(index).unwrap();
            line.push_str(if index == 0 { "alpha\n" } else { "yes\n" });
            Ok(line.len())
        });
        let client = LocalStdioClient {
            input: Arc::new(broker),
        };
        // No request means no stdin read, even after giving the worker time to run.
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        let response = client
            .execute(read_request(1, ExecutionBudget::default()))
            .await
            .unwrap();
        assert_eq!(response.result, ConsoleResult::Input("alpha\n".into()));
        assert_eq!(reader_started.try_recv().unwrap(), 0);
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert_eq!(reads.load(Ordering::SeqCst), 1, "broker must not prefetch");
        let approval = client.read_prompt_line().await.unwrap();
        assert_eq!(approval, "yes\n");
        assert_eq!(reads.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn queued_read_obeys_its_deadline_while_another_read_waits() {
        let (_sender, receiver) = mpsc::channel(1);
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
        let outcome = tokio::time::timeout(
            Duration::from_secs(1),
            client.execute(read_request(
                2,
                ExecutionBudget::start(Budget {
                    time: Some(TimeBudget { max_millis: 25 }),
                    ..Budget::default()
                }),
            )),
        )
        .await;
        pending.abort();
        let _ = pending.await;
        let error = outcome
            .expect("waiting for the input lock must honor the deadline")
            .expect_err("queued request must expire");
        assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_read_does_not_consume_the_next_line() {
        let (sender, receiver) = mpsc::channel(1);
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
            .await
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

    #[tokio::test(flavor = "current_thread")]
    async fn scoped_read_all_cancellation_preserves_already_consumed_lines() {
        use crate::execution::{CancellationReason, ExecutionScope, ExternalOutcome};
        let (sender, receiver) = mpsc::channel(1);
        let client = LocalStdioClient::with_input(receiver);
        let scope = ExecutionScope::new();
        let registration = scope
            .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
            .unwrap();
        registration.begin_dispatch().unwrap();
        let mut request = read_request(1, ExecutionBudget::default());
        request.operation = ConsoleOperation::ReadAllStdin;
        request.authority.grants = vec![HostActionGrant::allow("Console", "stdin_read_all")];
        let pending = tokio::spawn({
            let client = client.clone();
            let context = registration.context().clone();
            async move { client.execute_scoped(request, &context).await }
        });
        sender
            .send(ConsoleInputEvent::Line("first\n".into()))
            .await
            .unwrap();
        // Capacity is returned only when the broker consumes the first line.
        let permit = sender.reserve().await.unwrap();
        drop(permit);
        scope
            .cancel_source()
            .stop(CancellationReason::Interrupt)
            .unwrap();
        let error = tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert_eq!(error.code, HostErrorCode::Cancelled);
        registration
            .complete(ExternalOutcome::Unknown, vec![])
            .unwrap();
        scope.finish_body(true).unwrap();
        scope.join().await.unwrap();
        let next = client
            .execute(read_request(2, ExecutionBudget::default()))
            .await
            .unwrap();
        assert_eq!(next.result, ConsoleResult::Input("first\n".into()));
        sender
            .send(ConsoleInputEvent::Line("second\n".into()))
            .await
            .unwrap();
        assert_eq!(client.read_prompt_line().await.unwrap(), "second\n");
    }
}
