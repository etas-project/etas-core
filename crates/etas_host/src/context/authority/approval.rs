use crate::{HostActionGrant, HostRequestId, TraceContext};

#[derive(Clone, Debug, PartialEq)]
pub struct ApprovalRequest {
    pub id: HostRequestId,
    pub reason: String,
    pub requested_grants: Vec<HostActionGrant>,
    pub trace: TraceContext,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ApprovalDecision {
    Approved { grant: ApprovalGrant },
    Denied { reason: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApprovalResponse {
    pub id: HostRequestId,
    pub decision: ApprovalDecision,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApprovalGrant {
    pub id: HostRequestId,
    pub grants: Vec<HostActionGrant>,
}
