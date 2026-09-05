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
}
