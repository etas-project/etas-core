use etas_std::StdSymbolId;

use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostRequestId, HostSchema, HostValue,
    TraceContext,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolRef {
    /// Provider-facing model tool name. This must remain compatible with model
    /// adapter naming restrictions.
    pub name: String,
    /// Canonical language/package identity for host routing, policy, and
    /// checkpoints. This may contain module/package path separators.
    pub qualified_name: Option<String>,
    pub std_symbol: Option<StdSymbolId>,
}

impl ToolRef {
    pub fn source(name: impl Into<String>, qualified_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            qualified_name: Some(qualified_name.into()),
            std_symbol: None,
        }
    }

    pub fn external(name: impl Into<String>, qualified_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            qualified_name: Some(qualified_name.into()),
            std_symbol: None,
        }
    }

    pub fn std(name: impl Into<String>, symbol: StdSymbolId) -> Self {
        Self {
            name: name.into(),
            qualified_name: None,
            std_symbol: Some(symbol),
        }
    }

    pub fn anonymous_test(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            qualified_name: None,
            std_symbol: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolSchema {
    pub tool: ToolRef,
    pub input: HostSchema,
    pub output: Option<HostSchema>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolRequest {
    pub id: HostRequestId,
    pub tool: ToolRef,
    pub args: HostValue,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolResponse {
    pub id: HostRequestId,
    pub result: Result<HostValue, HostError>,
}
