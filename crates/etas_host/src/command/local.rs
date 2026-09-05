mod output;
mod process_tree;
mod supervisor;

use std::{future::Future, pin::Pin, process::Stdio};

use tokio::process::Command as TokioCommand;

use crate::{
    ActionInstance, CommandClient, CommandRequest, CommandResponse, HostError, HostErrorCode,
    SandboxBroker, WorkspaceRegionRegistry,
};

use self::{process_tree::ProcessTreeController, supervisor::SupervisedCommand};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandExecutionPolicy {
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}

impl CommandExecutionPolicy {
    pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 1024 * 1024;

    pub const fn new(max_stdout_bytes: usize, max_stderr_bytes: usize) -> Self {
        Self {
            max_stdout_bytes,
            max_stderr_bytes,
        }
    }
}

impl Default for CommandExecutionPolicy {
    fn default() -> Self {
        Self::new(
            Self::DEFAULT_MAX_OUTPUT_BYTES,
            Self::DEFAULT_MAX_OUTPUT_BYTES,
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalCommandClient {
    policy: CommandExecutionPolicy,
    regions: WorkspaceRegionRegistry,
}

impl LocalCommandClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: CommandExecutionPolicy) -> Self {
        Self {
            policy,
            regions: WorkspaceRegionRegistry::default(),
        }
    }

    pub fn with_regions(regions: WorkspaceRegionRegistry) -> Self {
        Self {
            policy: CommandExecutionPolicy::default(),
            regions,
        }
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
        request.budget.check_time()?;
        let deadline = request.budget.deadline()?;

        let mut command = TokioCommand::new(program);
        command.args(request.argv.iter().skip(1));
        command.env_clear();
        for (key, value) in &request.env {
            command.env(key, value);
        }
        let cwd = request
            .cwd
            .as_ref()
            .map(|path| self.regions.resolve(path, false))
            .transpose()?;
        if let Some(cwd) = &cwd {
            command.current_dir(cwd.absolute());
        }
        command.stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        ProcessTreeController::configure(&mut command);

        let supervised = SupervisedCommand::spawn(
            command,
            request.stdin,
            deadline,
            self.policy,
            program.clone(),
        )?;
        let output = supervised.wait().await?;

        Ok(CommandResponse {
            id: request.id,
            result: Ok(output),
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
