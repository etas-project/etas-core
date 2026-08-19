use crate::{
    ApprovalRequest, AuthorityContext, HostError, HostRequestId, HostRequestKind, TraceContext,
};

#[derive(Clone, Debug, PartialEq)]
pub enum TraceEvent {
    HostRequestStarted {
        id: HostRequestId,
        kind: HostRequestKind,
        authority: Box<AuthorityContext>,
        trace: TraceContext,
    },
    HostRequestFinished {
        id: HostRequestId,
        outcome: HostOutcome,
    },
    ApprovalRequested {
        request: ApprovalRequest,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostOutcome {
    Succeeded,
    Failed(HostError),
    Cancelled { reason: String },
}
