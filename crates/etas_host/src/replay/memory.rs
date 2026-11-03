use super::{ReplayKey, ReplayKeyError, ReplayKeyErrorKind, ReplayKeyId, ReplayKeyProvider};
use std::{
    collections::BTreeMap,
    num::NonZeroUsize,
    sync::{Arc, Mutex, OnceLock},
};

pub struct MemoryReplayKeyProvider {
    keys: Mutex<BTreeMap<ReplayKeyId, Arc<ReplayKey>>>,
    capacity: NonZeroUsize,
}

impl MemoryReplayKeyProvider {
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            keys: Mutex::new(BTreeMap::new()),
            capacity,
        }
    }

    pub fn remove(&self, id: ReplayKeyId) -> Result<bool, ReplayKeyError> {
        Ok(self
            .keys
            .lock()
            .map_err(|_| poisoned())?
            .remove(&id)
            .is_some())
    }
}

impl std::fmt::Debug for MemoryReplayKeyProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryReplayKeyProvider")
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

impl ReplayKeyProvider for MemoryReplayKeyProvider {
    fn validate_authority(&self, _: &crate::AuthorityContext) -> Result<(), ReplayKeyError> {
        Ok(())
    }

    fn create(&self) -> Result<Arc<ReplayKey>, ReplayKeyError> {
        ReplayKey::generate().map(Arc::new)
    }

    fn retain(&self, key: &Arc<ReplayKey>) -> Result<(), ReplayKeyError> {
        let mut keys = self.keys.lock().map_err(|_| poisoned())?;
        if keys.contains_key(&key.id()) {
            return Ok(());
        }
        if keys.len() >= self.capacity.get() {
            return Err(ReplayKeyError::new(
                ReplayKeyErrorKind::Capacity,
                "checkpoint replay-key provider capacity exhausted",
            ));
        }
        keys.insert(key.id(), key.clone());
        Ok(())
    }

    fn resolve(&self, id: ReplayKeyId) -> Result<Arc<ReplayKey>, ReplayKeyError> {
        self.keys
            .lock()
            .map_err(|_| poisoned())?
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                ReplayKeyError::new(
                    ReplayKeyErrorKind::Missing,
                    format!("checkpoint replay key {id} is unavailable"),
                )
            })
    }
}

fn poisoned() -> ReplayKeyError {
    ReplayKeyError::new(
        ReplayKeyErrorKind::Unavailable,
        "checkpoint replay-key provider lock poisoned",
    )
}

/// Default embedded provider is process-local and bounded; resolve never creates keys.
pub fn default_replay_key_provider() -> Arc<dyn ReplayKeyProvider> {
    static PROVIDER: OnceLock<Arc<MemoryReplayKeyProvider>> = OnceLock::new();
    PROVIDER
        .get_or_init(|| {
            Arc::new(MemoryReplayKeyProvider::new(
                NonZeroUsize::MIN.saturating_add(4095),
            ))
        })
        .clone()
}
