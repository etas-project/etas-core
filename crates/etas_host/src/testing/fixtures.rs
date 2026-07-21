use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{HostError, HostErrorCode, WorkspaceRoot};

#[derive(Debug)]
pub struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    pub fn create(name: &str) -> Result<Self, HostError> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                HostError::new(
                    HostErrorCode::InvalidRequest,
                    "system clock is before UNIX epoch",
                )
                .with_detail("error", error.to_string())
            })?
            .as_nanos();
        let root = std::env::temp_dir().join(format!("etas-host-{name}-{nanos}"));
        fs::create_dir(&root).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to create test workspace",
            )
            .with_detail("path", root.display().to_string())
            .with_detail("error", error.to_string())
        })?;
        Ok(Self { root })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn root(&self) -> Result<WorkspaceRoot, HostError> {
        WorkspaceRoot::new(&self.root)
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
