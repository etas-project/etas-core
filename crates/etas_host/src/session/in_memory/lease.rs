use std::collections::btree_map::Entry;

use super::*;
use crate::session::{SessionLeaseOperation as Op, SessionLeaseResult as ResultValue, lease};

#[derive(Debug, Default)]
pub(super) struct RoutingLeases {
    active: BTreeMap<String, crate::session::SessionLease>,
    retired: BTreeMap<String, u64>,
}

impl InMemorySessionClient {
    pub(super) fn execute_lease(
        &self,
        request: Op,
        operation: &crate::execution::OperationContext,
        budget: &crate::ExecutionBudget,
    ) -> Result<SessionResult, HostError> {
        let mut leases = self.leases.write().map_err(lock_error)?;
        operation.signal().check()?;
        budget.check_time()?;
        let now = crate::storage::receipt::now()?;
        let result = match request {
            Op::Acquire(scope) => {
                let token = scope.issue(now)?;
                leases
                    .active
                    .retain(|_, lease| lease.scope().expires_at() > now);
                leases.retired.retain(|_, expiry| *expiry > now);
                if leases.retired.contains_key(scope.key()) {
                    return Err(lease::inactive());
                }
                match leases.active.entry(scope.session().id.clone()) {
                    Entry::Occupied(_) => return Err(lease::held()),
                    Entry::Vacant(entry) => entry.insert(token.clone()),
                };
                ResultValue::Acquired(token)
            }
            Op::Handoff(previous) => {
                let next = previous.scope().issue(now)?;
                let token = leases
                    .active
                    .get_mut(&previous.session().id)
                    .filter(|token| **token == previous)
                    .ok_or_else(lease::inactive)?;
                *token = next.clone();
                ResultValue::Transferred(next)
            }
            Op::Release(token) => {
                match leases.active.entry(token.session().id.clone()) {
                    Entry::Occupied(entry)
                        if *entry.get() == token && token.scope().expires_at() > now =>
                    {
                        entry.remove();
                    }
                    _ => return Err(lease::inactive()),
                }
                leases
                    .retired
                    .insert(token.scope().key().to_owned(), token.scope().expires_at());
                ResultValue::Released
            }
            Op::Revoke(scope) => {
                if let Entry::Occupied(entry) = leases.active.entry(scope.session().id.clone())
                    && entry.get().scope() == &scope
                {
                    entry.remove();
                }
                if scope.expires_at() > now {
                    leases
                        .retired
                        .insert(scope.key().to_owned(), scope.expires_at());
                }
                ResultValue::Revoked
            }
        };
        Ok(SessionResult::Lease(result))
    }
}
