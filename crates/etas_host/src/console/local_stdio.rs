use std::{
    future::Future,
    io::{Read, Write},
    pin::Pin,
};

use crate::{ActionInstance, HostError, HostErrorCode};

use super::{ConsoleClient, ConsoleOperation, ConsoleRequest, ConsoleResponse, ConsoleResult};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalStdioClient;

impl LocalStdioClient {
    pub fn new() -> Self {
        Self
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

    fn execute_local(request: ConsoleRequest) -> Result<ConsoleResponse, HostError> {
        Self::require_console_authority(&request)?;
        let result = match request.operation {
            ConsoleOperation::ReadAllStdin => {
                let mut input = String::new();
                std::io::stdin()
                    .read_to_string(&mut input)
                    .map_err(|error| {
                        HostError::new(HostErrorCode::ProviderUnavailable, "failed to read stdin")
                            .with_detail("error", error.to_string())
                    })?;
                ConsoleResult::Input(input)
            }
            ConsoleOperation::ReadLineStdin => {
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).map_err(|error| {
                    HostError::new(HostErrorCode::ProviderUnavailable, "failed to read stdin")
                        .with_detail("error", error.to_string())
                })?;
                ConsoleResult::Input(input)
            }
            ConsoleOperation::WriteStdout { text, newline } => {
                let mut stdout = std::io::stdout().lock();
                if newline {
                    writeln!(stdout, "{text}")
                } else {
                    write!(stdout, "{text}")
                }
                .and_then(|_| stdout.flush())
                .map_err(|error| {
                    HostError::new(HostErrorCode::ProviderUnavailable, "failed to write stdout")
                        .with_detail("error", error.to_string())
                })?;
                ConsoleResult::Written
            }
            ConsoleOperation::WriteStderr { text, newline } => {
                let mut stderr = std::io::stderr().lock();
                if newline {
                    writeln!(stderr, "{text}")
                } else {
                    write!(stderr, "{text}")
                }
                .and_then(|_| stderr.flush())
                .map_err(|error| {
                    HostError::new(HostErrorCode::ProviderUnavailable, "failed to write stderr")
                        .with_detail("error", error.to_string())
                })?;
                ConsoleResult::Written
            }
        };
        Ok(ConsoleResponse {
            id: request.id,
            result,
        })
    }
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
        Box::pin(async move { Self::execute_local(request) })
    }
}
