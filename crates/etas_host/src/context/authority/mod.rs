pub mod action_grant;
pub mod approval;
pub mod policy;

pub use action_grant::{ActionArgPattern, ActionInstance, ActionPattern, HostActionGrant};
pub use approval::{ApprovalDecision, ApprovalGrant, ApprovalRequest, ApprovalResponse};
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

    pub fn with_approvals(&self) -> Self {
        let mut result = self.clone();
        for approval in &self.approvals {
            for grant in &approval.grants {
                if !result.grants.contains(grant) {
                    result.grants.push(grant.clone());
                }
            }
        }
        result
    }

    /// Preserve exact grant predicates, including overlapping argument patterns.
    /// Receiver approvals constrain its capabilities without becoming evidence
    /// that this caller has obtained an approval.
    pub fn conjoin(&self, restriction: &Self) -> Self {
        Self {
            grants: vec![HostActionGrant::Conjunction(vec![
                self.grants.clone(),
                restriction.grants.clone(),
            ])],
            approvals: self.approvals.clone(),
            sandbox: self.sandbox.conjoin(&restriction.sandbox),
            policy: self.policy.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostRequestId, HostValue};

    #[test]
    fn conjunction_preserves_overlapping_patterns_and_caller_approval_evidence() {
        let mut caller = AuthorityContext::deny_all();
        caller.grants = vec![HostActionGrant::allow_with_args(
            "Fs",
            "read",
            vec![ActionArgPattern::Prefix(vec!["reports".into()])],
        )];
        caller.approvals.push(ApprovalGrant {
            id: HostRequestId(1),
            grants: vec![HostActionGrant::allow("Console", "stdout_write")],
        });
        let mut owner = AuthorityContext::deny_all();
        owner.grants = vec![HostActionGrant::allow_with_args(
            "Fs",
            "read",
            vec![ActionArgPattern::Prefix(vec![
                "reports".into(),
                "q1".into(),
            ])],
        )];
        owner.approvals.push(ApprovalGrant {
            id: HostRequestId(2),
            grants: vec![HostActionGrant::allow("Fs", "write")],
        });
        let combined = caller.with_approvals().conjoin(&owner.with_approvals());
        assert!(combined.allows(&ActionInstance::new(
            "Fs",
            "read",
            vec![HostValue::List(vec![
                HostValue::String("reports".into()),
                HostValue::String("q1".into())
            ]),]
        )));
        assert!(!combined.allows(&ActionInstance::new(
            "Fs",
            "read",
            vec![HostValue::String("reports.q2".into()),]
        )));
        assert!(!combined.allows(&ActionInstance::new("Fs", "write", vec![])));
        assert_eq!(combined.approvals, caller.approvals);
        let mut another_owner = AuthorityContext::deny_all();
        another_owner.grants = vec![HostActionGrant::allow("Console", "stdout_write")];
        let constrained = combined.conjoin(&another_owner);
        assert!(!constrained.allows(&ActionInstance::new("Console", "stdout_write", vec![])));
    }
}
