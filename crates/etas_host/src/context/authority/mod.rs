pub mod action_grant;
pub mod approval;
pub mod policy;

pub use action_grant::{ActionArgPattern, ActionInstance, ActionPattern, HostActionGrant};
pub use approval::{ApprovalDecision, ApprovalGrant, ApprovalRequest};
pub use policy::PolicyContext;

use crate::SandboxPolicy;

#[derive(Clone, Debug, PartialEq)]
pub struct AuthorityContext {
    pub grants: Vec<HostActionGrant>,
    pub approvals: Vec<ApprovalGrant>,
    pub sandbox: SandboxPolicy,
    pub policy: PolicyContext,
}

impl AuthorityContext {
    pub fn deny_all() -> Self {
        Self {
            grants: Vec::new(),
            approvals: Vec::new(),
            sandbox: SandboxPolicy::deny_all(),
            policy: PolicyContext::default(),
        }
    }

    pub fn allows(&self, action: &ActionInstance) -> bool {
        self.grants.iter().any(|grant| grant.allows(action))
    }
}
