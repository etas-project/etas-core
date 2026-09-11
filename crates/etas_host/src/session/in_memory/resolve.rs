use super::*;
use crate::session::{SessionResolveReceipt, SessionWriteReceipt};

pub(super) fn apply(
    sessions: &mut BTreeMap<String, SessionState>,
    config: SessionConfig,
    operation: &crate::StorageOperationRef,
    limits: &crate::StorageLimits,
) -> Result<SessionWriteReceipt, HostError> {
    if sessions.get(&config.id).is_some_and(|s| {
        s.context_receipts.contains_key(operation.key.as_str())
            || s.retention_receipts.contains_key(operation.key.as_str())
    }) {
        return Err(crate::session::write::mismatch());
    }
    if let Some(existing) = sessions
        .get(&config.id)
        .and_then(|state| state.receipts.get(operation.key.as_str()))
    {
        if existing.operation() != operation {
            return Err(crate::session::write::mismatch());
        }
        return Ok(existing.clone());
    }
    let used = super::context::capacity(sessions, limits)?;
    let staged = match sessions.get(&config.id) {
        Some(existing) => {
            if existing.config != config {
                return Err(invalid_request(
                    "session identity already has a different configuration",
                ));
            }
            None
        }
        None => Some(SessionState {
            retention_receipts: BTreeMap::new(),
            context_receipts: BTreeMap::new(),
            published_context: None,
            history_key: crate::session::context::fence::HistoryKey::new()?,
            context_version: 0,
            generation: crate::storage::version::StoreGeneration::new()?,
            storage_generation: crate::storage::version::StoreGeneration::new()?,
            receipts: BTreeMap::new(),
            config: config.clone(),
            messages: BTreeMap::new(),
            last_ordinal: -1,
            dedup: BTreeMap::new(),
            summary: None,
        }),
    };
    let state = staged
        .as_ref()
        .or_else(|| sessions.get(&config.id))
        .ok_or_else(|| invalid_request("session resolve staging state is missing"))?;
    let receipt = SessionWriteReceipt::Resolve(SessionResolveReceipt {
        operation: operation.clone(),
        session: SessionRef {
            id: config.id.clone(),
        },
        created: staged.is_some(),
        generation: crate::session::SessionGeneration::issue(
            &config.id,
            state.storage_generation.as_str(),
        )?,
        durability: crate::StorageDurability::Volatile,
    });
    crate::storage::receipt_budget::admit(limits, used, receipt.charge(&config.id)?)?;
    if let Some(mut state) = staged {
        state
            .receipts
            .insert(operation.key.as_str().to_owned(), receipt.clone());
        sessions.insert(config.id, state);
    } else if let Some(state) = sessions.get_mut(&config.id) {
        state
            .receipts
            .insert(operation.key.as_str().to_owned(), receipt.clone());
    } else {
        return Err(invalid_request("session disappeared within resolve lock"));
    }
    Ok(receipt)
}
