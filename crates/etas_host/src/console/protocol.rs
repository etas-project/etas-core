use crate::{AuthorityContext, Budget, HostRequestId, TraceContext};

#[derive(Clone, Debug, PartialEq)]
pub struct ConsoleRequest {
    pub id: HostRequestId,
    pub operation: ConsoleOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsoleOperation {
    ReadAllStdin,
    ReadLineStdin,
    WriteStdout { text: String, newline: bool },
    WriteStderr { text: String, newline: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConsoleResponse {
    pub id: HostRequestId,
    pub result: ConsoleResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsoleResult {
    Input(String),
    Written,
}
