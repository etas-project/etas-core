use super::*;
use crate::session::{SessionLeaseOperation as Op, SessionLeaseResult as ResultValue, lease};

impl SessionDatabase<'_> {
    pub(super) fn execute_lease(&mut self, request: Op) -> Result<SessionResult, HostError> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sqlite_error)?;
        let now = crate::storage::receipt::now()?;
        let result = match request {
            Op::Acquire(scope) => {
                let token = scope.issue(now)?;
                tx.execute(
                    "DELETE FROM session_routing_leases WHERE expires<=?1",
                    [now],
                )
                .map_err(sqlite_error)?;
                let retired = tx.query_row(
                    "SELECT 1 FROM session_routing_leases WHERE scope=?1 AND generation IS NULL",
                    [scope.key()], |_| Ok(()),
                ).optional().map_err(sqlite_error)?.is_some();
                if retired {
                    return Err(lease::inactive());
                }
                let inserted = tx.execute(
                    "INSERT INTO session_routing_leases (session_id, scope, generation, expires)
                     VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
                    params![scope.session().id, scope.key(), token.generation().as_token(), scope.expires_at()],
                ).map_err(sqlite_error)?;
                if inserted == 0 {
                    return Err(lease::held());
                }
                ResultValue::Acquired(token)
            }
            Op::Handoff(previous) => {
                let scope = previous.scope();
                let next = scope.issue(now)?;
                let changed = tx
                    .execute(
                        "UPDATE session_routing_leases SET generation=?1
                     WHERE session_id=?2 AND scope=?3 AND generation=?4 AND expires>?5",
                        params![
                            next.generation().as_token(),
                            scope.session().id,
                            scope.key(),
                            previous.generation().as_token(),
                            now
                        ],
                    )
                    .map_err(sqlite_error)?;
                if changed == 0 {
                    return Err(lease::inactive());
                }
                ResultValue::Transferred(next)
            }
            Op::Release(token) => {
                let scope = token.scope();
                let removed = tx
                    .execute(
                        "UPDATE session_routing_leases SET generation=NULL
                     WHERE session_id=?1 AND scope=?2 AND generation=?3 AND expires>?4",
                        params![
                            scope.session().id,
                            scope.key(),
                            token.generation().as_token(),
                            now
                        ],
                    )
                    .map_err(sqlite_error)?;
                if removed == 0 {
                    return Err(lease::inactive());
                }
                ResultValue::Released
            }
            Op::Revoke(scope) => {
                // Keep the retired scope until expiry, including when its acquire
                // has not reached the backend yet.
                if scope.expires_at() > now {
                    tx.execute(
                        "INSERT INTO session_routing_leases (session_id, scope, generation, expires)
                         VALUES (?1, ?2, NULL, ?3)
                         ON CONFLICT(scope) DO UPDATE SET generation=NULL",
                        params![scope.session().id, scope.key(), scope.expires_at()],
                    ).map_err(sqlite_error)?;
                }
                ResultValue::Revoked
            }
        };
        if let Some(operation) = self.operation {
            operation.check()?;
        }
        tx.commit().map_err(sqlite_error)?;
        Ok(SessionResult::Lease(result))
    }
}
