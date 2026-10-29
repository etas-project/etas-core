use etas_code::definition::DefinitionRef;

use crate::{
    ActionInstance, AuthorityContext, ExecutionBudget, HostRequestId, HostTraceFieldSensitivity,
    HostTracePayload, HostTraceRequest, HostValue, TraceContext,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilerOperation {
    Check,
    Compile,
}

impl CompilerOperation {
    pub const fn action(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::Compile => "compile",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CompilerRequest {
    pub id: HostRequestId,
    pub operation: CompilerOperation,
    pub definition: DefinitionRef,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

impl CompilerRequest {
    pub fn action(&self) -> ActionInstance {
        ActionInstance::new(
            "Code",
            self.operation.action(),
            vec![HostValue::String(
                self.definition.identity().fingerprint().to_string(),
            )],
        )
    }
}

impl HostTraceRequest for CompilerRequest {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("compiler", format!("Code.{}", self.operation.action()))
            .with_field(
                "definition",
                HostValue::String(self.definition.identity().fingerprint().to_string()),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "revision",
                HostValue::String(self.definition.revision().key().fingerprint().to_string()),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "environment",
                HostValue::String(self.definition.revision().inputs().environment.to_string()),
                HostTraceFieldSensitivity::Public,
            )
            .with_field(
                "contract",
                HostValue::String(self.definition.revision().inputs().contract.to_string()),
                HostTraceFieldSensitivity::Public,
            )
    }
}
