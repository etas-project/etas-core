use std::{future::Future, pin::Pin, process::Stdio};

use tokio::{io::AsyncWriteExt, process::Command as TokioCommand};

use crate::{
    ActionInstance, CommandClient, CommandOutput, CommandRequest, CommandResponse, HostError,
    HostErrorCode, SandboxBroker,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalCommandClient;

impl LocalCommandClient {
    pub fn new() -> Self {
        Self
    }

    async fn execute_local(&self, request: CommandRequest) -> Result<CommandResponse, HostError> {
        let program = request.argv.first().ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "command argv must not be empty",
            )
        })?;
        let action = ActionInstance::new("Command", "run", Vec::new());
        if !request.authority.allows(&action) {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "command execution requires a checked Command.run grant",
            )
            .with_detail("program", program.clone()));
        }
        SandboxBroker::new(request.authority.sandbox.clone()).check_command(program)?;

        let mut command = TokioCommand::new(program);
        command.args(request.argv.iter().skip(1));
        command.env_clear();
        for (key, value) in &request.env {
            command.env(key, value);
        }
        if let Some(cwd) = &request.cwd {
            command.current_dir(cwd.absolute());
        }
        command.stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to spawn command",
            )
            .with_detail("program", program.clone())
            .with_detail("error", error.to_string())
        })?;
        if let Some(stdin) = request.stdin
            && let Some(child_stdin) = child.stdin.as_mut()
        {
            child_stdin.write_all(&stdin).await.map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to write command stdin",
                )
                .with_detail("program", program.clone())
                .with_detail("error", error.to_string())
            })?;
        }
        let output = child.wait_with_output().await.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to wait for command output",
            )
            .with_detail("program", program.clone())
            .with_detail("error", error.to_string())
        })?;

        Ok(CommandResponse {
            id: request.id,
            result: Ok(CommandOutput {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: output.stdout,
                stderr: output.stderr,
            }),
        })
    }
}

impl CommandClient for LocalCommandClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<CommandResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: CommandRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move { self.execute_local(request).await })
    }
}
