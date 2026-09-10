use super::*;
use crate::session::maintenance::{cutoff, expired, rejected};
use crate::session::{
    RetentionProgress, SessionGeneration, SessionMaintenanceClient, SessionMaintenanceOperation,
    SessionMaintenanceRequest, SessionMaintenanceResponse, SessionMaintenanceResult,
    SessionRetentionIntent, SessionRetentionReceipt,
};
use crate::{ReceiptLookup, StorageDurability};

impl SessionMaintenanceClient for InMemorySessionClient {
    type MaintainFuture<'a> =
        Pin<Box<dyn Future<Output = Result<SessionMaintenanceResponse, HostError>> + Send + 'a>>;
    fn maintain(&self, request: SessionMaintenanceRequest) -> Self::MaintainFuture<'_> {
        Box::pin(self.dispatch_maintenance(request, None))
    }
}
impl InMemorySessionClient {
    pub async fn maintain_scoped(
        &self,
        request: SessionMaintenanceRequest,
        context: &crate::execution::OperationContext,
    ) -> Result<SessionMaintenanceResponse, HostError> {
        self.dispatch_maintenance(request, Some(context)).await
    }
    async fn dispatch_maintenance(
        &self,
        request: SessionMaintenanceRequest,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<SessionMaintenanceResponse, HostError> {
        let bytes = request.validate(&self.limits)?;
        let client = self.clone();
        let id = request.id;
        let intent = match &request.operation {
            SessionMaintenanceOperation::Retain(intent) => Some(intent.operation.clone()),
            SessionMaintenanceOperation::Reconcile { .. } => None,
        };
        let result = self
            .executor
            .execute(
                context,
                request.id,
                request.trace.clone(),
                request.budget.clone(),
                bytes,
                move |context| {
                    let result = client.execute_maintenance(request.operation, &|| {
                        request.budget.check_time()?;
                        context.signal().check()
                    });
                    Ok(SessionMaintenanceResponse {
                        id: request.id,
                        result,
                    })
                },
            )
            .await;
        match (result, intent) {
            (Err(error), Some(operation)) => Ok(SessionMaintenanceResponse {
                id,
                result: Ok(SessionMaintenanceResult::Outcome(
                    crate::WriteOutcome::from_dispatch_error(
                        error,
                        operation,
                        crate::session::SessionRetentionRejection::Rejected,
                    ),
                )),
            }),
            (result, _) => result.map_err(crate::execution::DispatchError::into_host_error),
        }
    }
    fn execute_maintenance(
        &self,
        request: SessionMaintenanceOperation,
        check: &dyn Fn() -> Result<(), HostError>,
    ) -> Result<SessionMaintenanceResult, HostError> {
        check()?;
        let mut sessions = self.sessions.write().map_err(lock_error)?;
        let now = crate::storage::receipt::now()?;
        for state in sessions.values_mut() {
            state
                .retention_receipts
                .retain(|_, r| r.operation.key.expires_at() > now);
            state
                .receipts
                .retain(|_, r| r.operation().key.expires_at() > now);
            state
                .context_receipts
                .retain(|_, r| r.operation().key.expires_at() > now);
        }
        match request {
            SessionMaintenanceOperation::Reconcile { session, operation } => {
                operation.validate()?;
                if sessions.get(&session.id).is_some_and(|s| {
                    s.receipts.contains_key(operation.key.as_str())
                        || s.context_receipts.contains_key(operation.key.as_str())
                }) {
                    return Err(crate::session::write::mismatch());
                }
                let found = if operation.key.is_expired()? {
                    ReceiptLookup::Expired
                } else {
                    match sessions
                        .get(&session.id)
                        .and_then(|s| s.retention_receipts.get(operation.key.as_str()))
                    {
                        Some(r) if r.operation == operation => {
                            r.check_result_size(&self.limits)?;
                            ReceiptLookup::Found(r.clone())
                        }
                        Some(_) => return Err(crate::session::write::mismatch()),
                        None => ReceiptLookup::Unresolved,
                    }
                };
                Ok(SessionMaintenanceResult::Receipt(found))
            }
            SessionMaintenanceOperation::Retain(intent) => {
                intent.validate(&self.limits)?;
                let operation = intent.operation.clone();
                let result = retain(&mut sessions, *intent, &self.limits, check);
                Ok(SessionMaintenanceResult::Outcome(match result {
                    Ok(receipt) => receipt.into_outcome(),
                    Err(error) => rejected(operation, error),
                }))
            }
        }
    }
}

fn retain(
    sessions: &mut BTreeMap<String, SessionState>,
    intent: SessionRetentionIntent,
    limits: &crate::StorageLimits,
    check: &dyn Fn() -> Result<(), HostError>,
) -> Result<SessionRetentionReceipt, HostError> {
    if sessions.get(&intent.session.id).is_some_and(|s| {
        s.receipts.contains_key(intent.operation.key.as_str())
            || s.context_receipts
                .contains_key(intent.operation.key.as_str())
    }) {
        return Err(crate::session::write::mismatch());
    }
    if let Some(receipt) = sessions
        .get(&intent.session.id)
        .and_then(|s| s.retention_receipts.get(intent.operation.key.as_str()))
    {
        if receipt.operation != intent.operation {
            return Err(crate::session::write::mismatch());
        }
        receipt.check_result_size(limits)?;
        return Ok(receipt.clone());
    }
    let used = super::context::capacity(sessions, limits)?;
    let state = sessions
        .get_mut(&intent.session.id)
        .ok_or_else(|| invalid_request("unresolved retention session"))?;
    let selection = intent
        .fence
        .retention_selection(&super::paging::history_state(state)?, limits)?;
    let cutoff = cutoff(selection.retained_after, &state.config.retention)?;
    let mut progress = RetentionProgress {
        scanned: 0,
        deleted_messages: 0,
        deleted_dedup_keys: 0,
        next_after: None,
    };
    let mut removals = Vec::new();
    let mut after = intent.after_ordinal;
    let mut bytes = 0usize;
    for (ordinal, message) in state
        .messages
        .range((intent.after_ordinal + 1)..(selection.upper + 1))
        .take(intent.scan_limit as usize)
    {
        check()?;
        bytes = bytes
            .checked_add(message.id.len())
            .and_then(|n| n.checked_add(message.created_at.len()))
            .and_then(|n| n.checked_add(message.dedup_key.as_ref().map_or(0, String::len)))
            .filter(|n| *n <= limits.max_result_bytes)
            .ok_or_else(crate::session::write::limit_error)?;
        progress.scanned += 1;
        after = *ordinal;
        if expired(&message.created_at, cutoff)? {
            progress.deleted_messages += 1;
            progress.deleted_dedup_keys += u32::from(message.dedup_key.is_some());
            removals.push((*ordinal, message.dedup_key.clone()));
        }
    }
    if progress.scanned == intent.scan_limit && after < selection.upper {
        progress.next_after = Some(after);
    }
    let generation = if removals.is_empty() {
        state.generation.clone()
    } else {
        crate::storage::version::StoreGeneration::new()?
    };
    let receipt = SessionRetentionReceipt {
        operation: intent.operation.clone(),
        session: intent.session.clone(),
        generation: SessionGeneration::issue(&intent.session.id, generation.as_str())?,
        progress,
        durability: StorageDurability::Volatile,
    };
    receipt.check_result_size(limits)?;
    crate::storage::receipt_budget::admit(limits, used, receipt.charge()?)?;
    check()?;
    for (ordinal, dedup) in removals {
        state.messages.remove(&ordinal);
        if let Some(key) = dedup {
            state.dedup.remove(&key);
        }
    }
    state.generation = generation;
    state
        .retention_receipts
        .insert(intent.operation.key.as_str().to_owned(), receipt.clone());
    Ok(receipt)
}
