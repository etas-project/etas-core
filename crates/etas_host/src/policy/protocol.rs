use crate::{ApprovalRequest, AuthorityContext, HostRequestId, HostValue, TraceContext};

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyEvaluationRequest {
    pub id: HostRequestId,
    pub policy_ref: HostValue,
    pub subject: PolicySubject,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolicySubject {
    pub kind: String,
    pub attributes: Vec<(String, HostValue)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyResponse {
    pub id: HostRequestId,
    pub decision: PolicyDecision,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
    RequireApproval { request: ApprovalRequest },
}
