use super::{ReplayKey, ReplayKeyError, ReplayKeyErrorKind, ReplayKeyId, ReplayKeyProvider};
use std::{path::Path, sync::Arc};

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::DirectoryReplayKeyProvider;

#[cfg(not(unix))]
#[derive(Debug)]
pub struct DirectoryReplayKeyProvider;

#[cfg(not(unix))]
impl DirectoryReplayKeyProvider {
    pub fn open(_: &Path) -> Result<Self, ReplayKeyError> {
        Err(unsupported())
    }
}

#[cfg(not(unix))]
impl ReplayKeyProvider for DirectoryReplayKeyProvider {
    fn validate_authority(&self, _: &crate::AuthorityContext) -> Result<(), ReplayKeyError> {
        Err(unsupported())
    }
    fn create(&self) -> Result<Arc<ReplayKey>, ReplayKeyError> {
        Err(unsupported())
    }
    fn retain(&self, _: &Arc<ReplayKey>) -> Result<(), ReplayKeyError> {
        Err(unsupported())
    }
    fn resolve(&self, _: ReplayKeyId) -> Result<Arc<ReplayKey>, ReplayKeyError> {
        Err(unsupported())
    }
}

#[cfg(not(unix))]
fn unsupported() -> ReplayKeyError {
    ReplayKeyError::new(
        ReplayKeyErrorKind::Unavailable,
        "a native protected checkpoint replay-key store is required on this platform",
    )
}
