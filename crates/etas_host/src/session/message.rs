use crate::{HostError, HostErrorCode, SessionMessage, StorageLimits};

impl SessionMessage {
    pub fn storage_size(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        let mut bytes = limits.value_size(&self.payload)?;
        if let Some(provenance) = &self.provenance {
            bytes = bytes
                .checked_add(limits.value_size(provenance)?)
                .ok_or_else(|| {
                    HostError::new(HostErrorCode::BudgetExceeded, "session size overflow")
                })?;
        }
        for text in [
            Some(&self.id),
            Some(&self.session.id),
            Some(&self.created_at),
            self.from.as_ref(),
            self.to.as_ref(),
            self.dedup_key.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            bytes = bytes.checked_add(text.len()).ok_or_else(|| {
                HostError::new(HostErrorCode::BudgetExceeded, "session size overflow")
            })?;
        }
        Ok(bytes)
    }
}
