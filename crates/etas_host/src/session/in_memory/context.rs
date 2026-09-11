use super::*;
use crate::ReceiptLookup;
use crate::session::{
    SessionContextEvidence, SessionContextPublication, SessionContextReceipt,
    SessionContextRejection, SessionPublishedContext, SessionWriteResult,
};

pub(super) fn capacity(
    sessions: &BTreeMap<String, SessionState>,
    limits: &crate::StorageLimits,
) -> Result<usize, HostError> {
    let count: usize = sessions
        .values()
        .map(|s| s.receipts.len() + s.context_receipts.len() + s.retention_receipts.len())
        .sum();
    if count >= limits.max_receipts {
        return Err(crate::session::write::limit_error());
    }
    crate::storage::receipt_budget::sum(sessions.iter().flat_map(|(id, s)| {
        s.receipts
            .values()
            .map(move |r| r.charge(id))
            .chain(s.context_receipts.values().map(move |r| r.charge(id)))
            .chain(s.retention_receipts.values().map(|r| r.charge()))
    }))
}

pub(super) fn publish(
    sessions: &mut BTreeMap<String, SessionState>,
    publication: SessionContextPublication,
    limits: &crate::StorageLimits,
) -> Result<SessionWriteResult, HostError> {
    publication.validate(limits)?;
    let operation = publication.operation.clone();
    let result = apply(sessions, publication, limits);
    Ok(SessionWriteResult::Context(match result {
        Ok(evidence) => evidence.into_outcome(),
        Err(error) => crate::session::context::rejected(operation, error),
    }))
}

fn apply(
    sessions: &mut BTreeMap<String, SessionState>,
    publication: SessionContextPublication,
    limits: &crate::StorageLimits,
) -> Result<SessionContextEvidence, HostError> {
    let operation = &publication.operation;
    operation
        .key
        .validate_window(limits.max_receipt_retention_seconds)?;
    if let Some(evidence) = lookup(sessions, &publication.session, operation)? {
        return Ok(evidence);
    }
    let used = capacity(sessions, limits)?;
    let state = sessions
        .get_mut(&publication.session.id)
        .ok_or_else(|| invalid_request("cannot publish context for an unresolved session"))?;
    let current = super::paging::history_state(state)?;
    let (evidence, update) = if !publication.fence.is_current(&current, limits)? {
        (
            SessionContextEvidence::NotCommitted {
                operation: operation.clone(),
                reason: SessionContextRejection::StaleHistory,
            },
            None,
        )
    } else {
        let version = state
            .context_version
            .checked_add(1)
            .filter(|n| *n <= i64::MAX as u64)
            .ok_or_else(|| invalid_request("session context version overflow"))?;
        let generation = crate::storage::version::StoreGeneration::new()?;
        let receipt = SessionContextReceipt {
            operation: operation.clone(),
            session: publication.session.clone(),
            generation: crate::session::SessionGeneration::issue(
                &publication.session.id,
                generation.as_str(),
            )?,
            context_version: version,
            durability: crate::StorageDurability::Volatile,
        };
        (
            SessionContextEvidence::Committed(receipt),
            Some((
                generation,
                SessionPublishedContext {
                    content: publication.content,
                    fence: publication.fence,
                    version,
                },
            )),
        )
    };
    crate::storage::receipt_budget::admit(limits, used, evidence.charge(&publication.session.id)?)?;
    if let Some((generation, context)) = update {
        state.generation = generation;
        state.context_version = context.version;
        state.published_context = Some(context);
        state.summary = None;
    }
    state
        .context_receipts
        .insert(operation.key.as_str().to_owned(), evidence.clone());
    Ok(evidence)
}

fn lookup(
    sessions: &BTreeMap<String, SessionState>,
    session: &SessionRef,
    operation: &crate::StorageOperationRef,
) -> Result<Option<SessionContextEvidence>, HostError> {
    let Some(state) = sessions.get(&session.id) else {
        return Ok(None);
    };
    if state.receipts.contains_key(operation.key.as_str())
        || state
            .retention_receipts
            .contains_key(operation.key.as_str())
    {
        return Err(crate::session::write::mismatch());
    }
    let found = state.context_receipts.get(operation.key.as_str());
    if found.is_some_and(|r| r.operation() != operation) {
        return Err(crate::session::write::mismatch());
    }
    Ok(found.cloned())
}

pub(super) fn reconcile(
    sessions: &BTreeMap<String, SessionState>,
    session: &SessionRef,
    operation: &crate::StorageOperationRef,
) -> Result<SessionWriteResult, HostError> {
    operation.validate()?;
    Ok(SessionWriteResult::ContextReceipt(
        if operation.key.is_expired()? {
            ReceiptLookup::Expired
        } else {
            match lookup(sessions, session, operation)? {
                Some(r) => ReceiptLookup::Found(r),
                None => ReceiptLookup::Unresolved,
            }
        },
    ))
}
