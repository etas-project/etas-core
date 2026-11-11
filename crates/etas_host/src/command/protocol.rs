use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostRequestId, TraceContext, WorkspacePathRef,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CommandRequest {
    pub id: HostRequestId,
    pub argv: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<WorkspacePathRef>,
    pub stdin: Option<Vec<u8>>,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandResponse {
    pub id: HostRequestId,
    pub result: Result<CommandOutput, HostError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub isolation: crate::CommandIsolationReport,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommandProcessRequest {
    pub id: HostRequestId,
    pub operation: CommandProcessOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandProcessOperation {
    Start {
        argv: Vec<String>,
        env: Vec<(String, String)>,
        cwd: Option<WorkspacePathRef>,
    },
    Reconnect {
        process: String,
    },
    Read {
        process: String,
        max_bytes: usize,
    },
    Write {
        process: String,
        bytes: Vec<u8>,
    },
    CloseStdin {
        process: String,
    },
    Stop {
        process: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandProcessResponse {
    pub id: HostRequestId,
    pub result: Result<CommandProcessPayload, HostError>,
    pub isolation: Option<crate::CommandIsolationReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandProcessPayload {
    Handle { id: String },
    Event(CommandProcessEvent),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandProcessEvent {
    Output {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    Exited {
        exit_code: i32,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    Stopped {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
}
