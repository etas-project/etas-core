mod directory;
mod key;
mod memory;
mod seal;

pub use directory::DirectoryReplayKeyProvider;
pub use key::{ReplayFingerprint, ReplayKey, ReplayKeyId};
pub use memory::{MemoryReplayKeyProvider, default_replay_key_provider};

use std::sync::Arc;

/// Credentials stay in the host provider, never in a checkpoint artifact.
pub trait ReplayKeyProvider: std::fmt::Debug + Send + Sync {
    fn validate_authority(&self, authority: &crate::AuthorityContext)
    -> Result<(), ReplayKeyError>;
    /// Start an invocation without persisting credentials for runs that never checkpoint.
    fn create(&self) -> Result<Arc<ReplayKey>, ReplayKeyError>;
    /// Make the key recoverable before publishing a checkpoint containing its identifier.
    fn retain(&self, key: &Arc<ReplayKey>) -> Result<(), ReplayKeyError>;
    fn resolve(&self, id: ReplayKeyId) -> Result<Arc<ReplayKey>, ReplayKeyError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayKeyErrorKind {
    Missing,
    Invalid,
    Unavailable,
    Capacity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplayKeyError {
    pub kind: ReplayKeyErrorKind,
    message: String,
}

impl ReplayKeyError {
    pub fn new(kind: ReplayKeyErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ReplayKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ReplayKeyError {}

#[cfg(test)]
mod tests;
