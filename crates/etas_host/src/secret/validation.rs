use super::{SecretOperation, SecretRequest};
use crate::{ActionInstance, HostError, HostErrorCode};

impl SecretRequest {
    /// Validate an unindexed host request before accessing secret material.
    pub fn validate(&self) -> Result<(), HostError> {
        self.budget.check_time()?;
        let action = match self.operation {
            SecretOperation::Read { .. } => "read",
            SecretOperation::HmacSha256 { .. } => "use",
        };
        // This protocol carries no checked static selector; payload keys cannot supply one.
        if !self
            .authority
            .allows(&ActionInstance::new("Secret", action, Vec::new()))
        {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "secret operation requires a current Secret action grant",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionArgPattern, AuthorityContext, Budget, ExecutionBudget, HostActionGrant,
        HostRequestId, SecretRef, TimeBudget, TraceContext, TraceId,
    };

    fn request(operation: SecretOperation) -> SecretRequest {
        SecretRequest {
            id: HostRequestId(1),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        }
    }

    #[test]
    fn provider_access_requires_the_matching_current_action_grant() {
        for (operation, action, other) in [
            (
                SecretOperation::Read {
                    key: "TOKEN".into(),
                },
                "read",
                "use",
            ),
            (
                SecretOperation::HmacSha256 {
                    key: SecretRef::new("TOKEN"),
                    body: vec![],
                },
                "use",
                "read",
            ),
        ] {
            let mut request = request(operation);
            assert_eq!(
                request.validate().unwrap_err().code,
                HostErrorCode::AuthorityDenied
            );
            request
                .authority
                .grants
                .push(HostActionGrant::allow("Secret", other));
            assert_eq!(
                request.validate().unwrap_err().code,
                HostErrorCode::AuthorityDenied
            );
            request
                .authority
                .grants
                .push(HostActionGrant::allow("Secret", action));
            request.validate().unwrap();
        }
    }

    #[test]
    fn runtime_key_cannot_stand_in_for_a_checked_static_selector() {
        let mut request = request(SecretOperation::Read {
            key: "TOKEN".into(),
        });
        request
            .authority
            .grants
            .push(HostActionGrant::allow_with_args(
                "Secret",
                "read",
                vec![ActionArgPattern::Any],
            ));
        assert_eq!(
            request.validate().unwrap_err().code,
            HostErrorCode::AuthorityDenied
        );
    }

    #[test]
    fn an_expired_request_cannot_access_the_provider() {
        let mut request = request(SecretOperation::Read {
            key: "TOKEN".into(),
        });
        request
            .authority
            .grants
            .push(HostActionGrant::allow("Secret", "read"));
        request.budget = ExecutionBudget::start(Budget {
            time: Some(TimeBudget { max_millis: 0 }),
            ..Default::default()
        });
        assert_eq!(
            request.validate().unwrap_err().code,
            HostErrorCode::BudgetExceeded
        );
    }
}
