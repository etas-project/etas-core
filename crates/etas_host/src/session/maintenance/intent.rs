use crate::session::{SessionHistoryFence, SessionRef};
use crate::{HostError, StorageLimits, StorageOperationKey, StorageOperationRef};

/// One bounded batch under a backend-issued retention cutoff. This intent is
/// not authority; it is used by an explicitly authorized runtime maintenance job.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionRetentionIntent {
    pub session: SessionRef,
    pub fence: SessionHistoryFence,
    pub after_ordinal: i64,
    pub scan_limit: u32,
    pub operation: StorageOperationRef,
}

impl SessionRetentionIntent {
    pub fn prepare(
        session: SessionRef,
        fence: SessionHistoryFence,
        after_ordinal: i64,
        scan_limit: u32,
        key: StorageOperationKey,
        limits: &StorageLimits,
    ) -> Result<Self, HostError> {
        let operation = operation_ref(&session, &fence, after_ordinal, scan_limit, key, limits)?;
        Ok(Self {
            session,
            fence,
            after_ordinal,
            scan_limit,
            operation,
        })
    }
    pub fn validate(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        let expected = operation_ref(
            &self.session,
            &self.fence,
            self.after_ordinal,
            self.scan_limit,
            self.operation.key.clone(),
            limits,
        )?;
        if expected != self.operation {
            return Err(super::invalid(
                "retention intent does not match prepared operation",
            ));
        }
        self.session
            .id
            .len()
            .checked_add(self.fence.as_token().len())
            .filter(|bytes| *bytes <= limits.max_value_bytes)
            .ok_or_else(crate::session::write::limit_error)
    }
}

fn operation_ref(
    session: &SessionRef,
    fence: &SessionHistoryFence,
    after: i64,
    limit: u32,
    key: StorageOperationKey,
    limits: &StorageLimits,
) -> Result<StorageOperationRef, HostError> {
    if session.id.is_empty()
        || session.id != fence.session_id()
        || after < -1
        || after > fence.selection_upper(limits)?
        || limit == 0
        || limit as usize > limits.max_scan_rows.min(limits.max_page_entries)
        || session
            .id
            .len()
            .checked_add(fence.as_token().len())
            .is_none_or(|n| n > limits.max_value_bytes)
    {
        return Err(super::invalid(
            "invalid or oversized session retention intent",
        ));
    }
    key.validate_window(limits.max_receipt_retention_seconds)?;
    let mut hash = blake3::Hasher::new_derive_key("etas.session.retention.v1");
    for text in [&session.id, fence.as_token()] {
        hash.update(&(text.len() as u64).to_le_bytes());
        hash.update(text.as_bytes());
    }
    hash.update(&after.to_le_bytes());
    hash.update(&limit.to_le_bytes());
    Ok(StorageOperationRef {
        key,
        request_fingerprint: hash.finalize().to_hex().to_string(),
    })
}
