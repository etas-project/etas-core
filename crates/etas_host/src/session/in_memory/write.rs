use super::*;
use crate::session::{
    SessionAppendReceipt, SessionVersion, SessionWriteOperation, SessionWriteReceipt,
    SessionWriteRequest, SessionWriteResponse, SessionWriteResult, append_operation_ref,
    resolve_operation_ref,
};
use crate::{ReceiptLookup, StorageDurability, WriteOutcome};

enum PendingMutation {
    Context(crate::StorageOperationRef),
    Write(crate::StorageOperationRef),
}

impl InMemorySessionClient {
    pub async fn write_scoped(
        &self,
        request: SessionWriteRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<SessionWriteResponse, HostError> {
        self.dispatch_write(request, Some(operation)).await
    }

    pub(super) async fn dispatch_write(
        &self,
        request: SessionWriteRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<SessionWriteResponse, HostError> {
        let bytes = request.storage_size(&self.limits)?;
        let id = request.id;
        let pending = match &request.operation {
            SessionWriteOperation::PublishContext(publication) => {
                Some(PendingMutation::Context(publication.operation.clone()))
            }
            SessionWriteOperation::Resolve { key, config } => Some(PendingMutation::Write(
                resolve_operation_ref(config, key.clone(), &self.limits)?,
            )),
            SessionWriteOperation::Append { key, message } => Some(PendingMutation::Write(
                append_operation_ref(message, key.clone(), &self.limits)?,
            )),
            SessionWriteOperation::Reconcile { operation, .. }
            | SessionWriteOperation::ReconcileContext { operation, .. } => {
                operation.validate()?;
                None
            }
        };
        let client = self.clone();
        let result = self
            .executor
            .execute(
                operation,
                request.id,
                request.trace.clone(),
                request.budget.clone(),
                bytes,
                move |_| {
                    Ok(SessionWriteResponse {
                        id: request.id,
                        result: client.execute_write(request.operation),
                    })
                },
            )
            .await;
        match (result, pending) {
            (Err(error), Some(PendingMutation::Context(operation))) => Ok(SessionWriteResponse {
                id,
                result: Ok(SessionWriteResult::Context(
                    WriteOutcome::from_dispatch_error(
                        error,
                        operation,
                        crate::session::SessionContextRejection::Rejected,
                    ),
                )),
            }),
            (Err(error), Some(PendingMutation::Write(operation))) => Ok(SessionWriteResponse {
                id,
                result: Ok(SessionWriteResult::Outcome(
                    WriteOutcome::from_dispatch_error(error, operation, |error| error),
                )),
            }),
            (result, _) => result.map_err(crate::execution::DispatchError::into_host_error),
        }
    }

    pub(super) fn execute_write(
        &self,
        request: SessionWriteOperation,
    ) -> Result<SessionWriteResult, HostError> {
        let mut sessions = self.sessions.write().map_err(lock_error)?;
        let now = crate::storage::receipt::now()?;
        for state in sessions.values_mut() {
            state
                .retention_receipts
                .retain(|_, receipt| receipt.operation.key.expires_at() > now);
            state
                .context_receipts
                .retain(|_, receipt| receipt.operation().key.expires_at() > now);
            state
                .receipts
                .retain(|_, receipt| receipt.operation().key.expires_at() > now);
        }
        match request {
            SessionWriteOperation::PublishContext(publication) => {
                super::context::publish(&mut sessions, *publication, &self.limits)
            }
            SessionWriteOperation::ReconcileContext { session, operation } => {
                super::context::reconcile(&sessions, &session, &operation)
            }
            SessionWriteOperation::Resolve { key, config } => {
                let operation = resolve_operation_ref(&config, key, &self.limits)?;
                let result = operation
                    .key
                    .validate_window(self.limits.max_receipt_retention_seconds)
                    .and_then(|()| {
                        super::resolve::apply(&mut sessions, config, &operation, &self.limits)
                    });
                Ok(SessionWriteResult::Outcome(match result {
                    Ok(receipt) => WriteOutcome::Committed(receipt),
                    Err(reason) => WriteOutcome::NotCommitted { operation, reason },
                }))
            }
            SessionWriteOperation::Reconcile { session, operation } => {
                operation.validate()?;
                if operation.key.is_expired()? {
                    return Ok(SessionWriteResult::Receipt(ReceiptLookup::Expired));
                }
                if sessions.get(&session.id).is_some_and(|s| {
                    s.context_receipts.contains_key(operation.key.as_str())
                        || s.retention_receipts.contains_key(operation.key.as_str())
                }) {
                    return Err(crate::session::write::mismatch());
                }
                let found = sessions
                    .get(&session.id)
                    .and_then(|state| state.receipts.get(operation.key.as_str()));
                if found.is_some_and(|receipt| receipt.operation() != &operation) {
                    return Err(crate::session::write::mismatch());
                }
                Ok(SessionWriteResult::Receipt(match found {
                    Some(receipt) => ReceiptLookup::Found(receipt.clone()),
                    None => ReceiptLookup::Unresolved,
                }))
            }
            SessionWriteOperation::Append { key, message } => {
                let operation = append_operation_ref(&message, key, &self.limits)?;
                let reject = |reason| {
                    SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                        operation: operation.clone(),
                        reason,
                    })
                };
                if let Err(error) = operation
                    .key
                    .validate_window(self.limits.max_receipt_retention_seconds)
                {
                    return Ok(reject(error));
                }
                if sessions.get(&message.session.id).is_some_and(|s| {
                    s.context_receipts.contains_key(operation.key.as_str())
                        || s.retention_receipts.contains_key(operation.key.as_str())
                }) {
                    return Ok(reject(crate::session::write::mismatch()));
                }
                if let Some(receipt) = sessions
                    .get(&message.session.id)
                    .and_then(|state| state.receipts.get(operation.key.as_str()))
                {
                    if receipt.operation() != &operation {
                        return Ok(reject(crate::session::write::mismatch()));
                    }
                    return Ok(SessionWriteResult::Outcome(WriteOutcome::Committed(
                        receipt.clone(),
                    )));
                }
                let used = match super::context::capacity(&sessions, &self.limits) {
                    Ok(used) => used,
                    Err(error) => return Ok(reject(error)),
                };
                let Some(state) = sessions.get_mut(&message.session.id) else {
                    return Ok(reject(invalid_request(
                        "cannot append message to an unresolved session",
                    )));
                };
                let (message, deduplicated, ordinal) = match prepare_append(state, *message) {
                    Ok(prepared) => prepared,
                    Err(error) => return Ok(reject(error)),
                };
                let version = SessionVersion::issue(
                    &message.session.id,
                    state.storage_generation.as_str(),
                    ordinal,
                )?;
                let receipt = SessionWriteReceipt::Append(SessionAppendReceipt {
                    operation,
                    version,
                    message_id: message.id.clone(),
                    deduplicated,
                    durability: StorageDurability::Volatile,
                });
                if let Err(error) = crate::storage::receipt_budget::admit(
                    &self.limits,
                    used,
                    receipt.charge(&message.session.id)?,
                ) {
                    return Ok(SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                        operation: receipt.operation().clone(),
                        reason: error,
                    }));
                }
                // All fallible validation precedes the single-lock publication.
                if !deduplicated {
                    apply_append(state, &message);
                }
                state
                    .receipts
                    .insert(receipt.operation().key.as_str().to_owned(), receipt.clone());
                Ok(SessionWriteResult::Outcome(WriteOutcome::Committed(
                    receipt,
                )))
            }
        }
    }
}

pub(super) fn prepare_append(
    state: &SessionState,
    message: SessionMessage,
) -> Result<(SessionMessage, bool, i64), HostError> {
    if message.id.is_empty() || message.session.id.is_empty() {
        return Err(invalid_request(
            "message and session identities must not be empty",
        ));
    }
    if let Some(key) = &message.dedup_key
        && let Some(existing_id) = state.dedup.get(key)
    {
        let (ordinal, existing) = state
            .messages
            .iter()
            .find(|(_, m)| &m.id == existing_id)
            .ok_or_else(|| {
                HostError::new(
                    HostErrorCode::SchemaMismatch,
                    "session dedup target is missing",
                )
            })?;
        crate::session::dedup::validate_replay(existing, &message)?;
        return Ok((existing.clone(), true, *ordinal));
    }
    if state.messages.values().any(|m| m.id == message.id) {
        return Err(invalid_request("session message id already exists"));
    }
    let ordinal = state
        .last_ordinal
        .checked_add(1)
        .filter(|n| *n < i64::MAX)
        .ok_or_else(|| invalid_request("session ordinal overflow"))?;
    Ok((message, false, ordinal))
}
pub(super) fn apply_append(state: &mut SessionState, message: &SessionMessage) {
    if let Some(key) = &message.dedup_key {
        state.dedup.insert(key.clone(), message.id.clone());
    }
    // prepare_append validates allocation before this infallible commit step.
    state.last_ordinal += 1;
    state.messages.insert(state.last_ordinal, message.clone());
}
