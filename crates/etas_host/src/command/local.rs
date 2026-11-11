mod cleanup;
mod cwd;
mod descriptors;
mod output;
mod process;
mod process_tree;
mod supervisor;

use std::{future::Future, pin::Pin, process::Stdio};

use tokio::process::Command as TokioCommand;

use crate::{
    ActionInstance, CommandClient, CommandProcessClient, CommandProcessOperation,
    CommandProcessPayload, CommandProcessRequest, CommandProcessResponse, CommandRequest,
    CommandResponse, HostError, HostErrorCode, SandboxBroker, WorkspaceRegionRegistry,
};

use self::{
    process::CommandProcessRegistry, process_tree::ProcessTreeController,
    supervisor::SupervisedCommand,
};

pub(crate) async fn execute_tool_process(
    mut command: TokioCommand,
    body: Vec<u8>,
    budget: &crate::ExecutionBudget,
    operation: Option<&crate::execution::OperationContext>,
    program: String,
) -> Result<crate::CommandOutput, HostError> {
    if let Some(operation) = operation {
        operation.signal().check()?;
    }
    budget.check_time()?;
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    ProcessTreeController::configure(&mut command);
    SupervisedCommand::spawn(
        command,
        Some(body),
        budget.deadline()?,
        CommandExecutionPolicy::default(),
        program,
        crate::sandbox::platform::PreparedIsolation::TrustedUnconfined,
        operation.cloned(),
    )?
    .wait(operation.map(|operation| operation.signal().clone()))
    .await
}

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

#[derive(Clone, Debug, Default)]
pub struct LocalCommandClient {
    policy: CommandExecutionPolicy,
    regions: WorkspaceRegionRegistry,
    processes: CommandProcessRegistry,
}

impl LocalCommandClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: CommandExecutionPolicy) -> Self {
        Self {
            policy,
            regions: WorkspaceRegionRegistry::default(),
            processes: CommandProcessRegistry::default(),
        }
    }

    pub fn with_regions(regions: WorkspaceRegionRegistry) -> Self {
        Self {
            policy: CommandExecutionPolicy::default(),
            regions,
            processes: CommandProcessRegistry::default(),
        }
    }

    async fn execute_local(
        &self,
        request: CommandRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<CommandResponse, HostError> {
        if let Some(operation) = operation {
            operation.signal().check()?;
        }
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
        let isolation =
            crate::sandbox::platform::PreparedIsolation::prepare(&request.authority.sandbox)?;
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
            .map(|path| self.regions.bind_directory(path))
            .transpose()?;
        if let Some(cwd) = &cwd {
            cwd::configure(&mut command, cwd)?;
        }
        command.stdin(if request.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        ProcessTreeController::configure(&mut command);

        if let Some(operation) = operation {
            operation.signal().check()?;
        }
        let supervised = SupervisedCommand::spawn(
            command,
            request.stdin,
            deadline,
            self.policy,
            program.clone(),
            isolation,
            operation.cloned(),
        )?;
        let output = supervised
            .wait(operation.map(|context| context.signal().clone()))
            .await?;

        Ok(CommandResponse {
            id: request.id,
            result: Ok(output),
        })
    }

    pub async fn execute_scoped(
        &self,
        request: CommandRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<CommandResponse, HostError> {
        let client = self.clone();
        operation
            .supervise(
                move |context| async move { client.execute_local(request, Some(&context)).await },
            )
            .await
    }

    async fn execute_process_local(
        &self,
        request: CommandProcessRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<CommandProcessResponse, HostError> {
        if let Some(operation) = operation {
            operation.signal().check()?;
        }
        request.budget.check_time()?;
        let action_name = match &request.operation {
            CommandProcessOperation::Start { .. } => "start",
            CommandProcessOperation::Reconnect { .. } => "reconnect",
            CommandProcessOperation::Read { .. } => "read",
            CommandProcessOperation::Write { .. } | CommandProcessOperation::CloseStdin { .. } => {
                "write"
            }
            CommandProcessOperation::Stop { .. } => "stop",
        };
        let action = ActionInstance::new("Command", action_name, Vec::new());
        if !request.authority.allows(&action) {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                format!(
                    "background command operation requires a checked Command.{action_name} grant"
                ),
            ));
        }
        let mut response_isolation = None;
        let result = match request.operation {
            CommandProcessOperation::Start { argv, env, cwd } => {
                let program = argv.first().ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::InvalidRequest,
                        "background command argv must not be empty",
                    )
                })?;
                SandboxBroker::new(request.authority.sandbox.clone()).check_command(program)?;
                let isolation = crate::sandbox::platform::PreparedIsolation::prepare(
                    &request.authority.sandbox,
                )?;
                let mut command = TokioCommand::new(program);
                command.args(argv.iter().skip(1));
                command.env_clear();
                for (key, value) in env {
                    command.env(key, value);
                }
                let cwd = cwd
                    .as_ref()
                    .map(|path| self.regions.bind_directory(path))
                    .transpose()?;
                if let Some(cwd) = &cwd {
                    cwd::configure(&mut command, cwd)?;
                }
                command.stdin(Stdio::piped());
                command.stdout(Stdio::piped());
                command.stderr(Stdio::piped());
                ProcessTreeController::configure(&mut command);
                let (id, isolation) = self
                    .processes
                    .start(
                        command,
                        request.budget.deadline()?,
                        self.policy,
                        program.clone(),
                        isolation,
                    )
                    .await?;
                response_isolation = Some(isolation);
                CommandProcessPayload::Handle { id }
            }
            CommandProcessOperation::Reconnect { process } => {
                let program = self.processes.program(&process)?;
                SandboxBroker::new(request.authority.sandbox.clone()).check_command(&program)?;
                CommandProcessPayload::Handle { id: process }
            }
            CommandProcessOperation::Read { process, max_bytes } => CommandProcessPayload::Event(
                self.processes
                    .read(&process, max_bytes, self.policy)
                    .await?,
            ),
            CommandProcessOperation::Write { process, bytes } => {
                self.processes.write(&process, bytes).await?;
                CommandProcessPayload::Unit
            }
            CommandProcessOperation::CloseStdin { process } => {
                self.processes.close_stdin(&process).await?;
                CommandProcessPayload::Unit
            }
            CommandProcessOperation::Stop { process } => {
                CommandProcessPayload::Event(self.processes.stop(&process).await?)
            }
        };
        Ok(CommandProcessResponse {
            id: request.id,
            result: Ok(result),
            isolation: response_isolation,
        })
    }

    pub async fn execute_process_scoped(
        &self,
        request: CommandProcessRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<CommandProcessResponse, HostError> {
        let client = self.clone();
        operation
            .supervise(move |context| async move {
                client.execute_process_local(request, Some(&context)).await
            })
            .await
    }
}

impl CommandClient for LocalCommandClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<CommandResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: CommandRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move { self.execute_local(request, None).await })
    }
}

impl CommandProcessClient for LocalCommandClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<CommandProcessResponse, Self::Error>> + Send + 'a>>;

    fn execute_process(&self, request: CommandProcessRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move { self.execute_process_local(request, None).await })
    }
}
