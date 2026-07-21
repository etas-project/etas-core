use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{HostError, HostErrorCode};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorkspaceRoot {
    pub canonical_root: PathBuf,
}

impl WorkspaceRoot {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, HostError> {
        let canonical_root = fs::canonicalize(root.as_ref()).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace root does not exist",
            )
            .with_detail("path", root.as_ref().display().to_string())
            .with_detail("error", error.to_string())
        })?;
        if !canonical_root.is_dir() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace root is not a directory",
            )
            .with_detail("path", canonical_root.display().to_string()));
        }
        Ok(Self { canonical_root })
    }

    pub fn resolve_existing(&self, path: impl AsRef<Path>) -> Result<WorkspacePath, HostError> {
        let relative = normalize_relative(path.as_ref())?;
        let candidate = self.canonical_root.join(&relative);
        let canonical = fs::canonicalize(&candidate).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace path does not exist",
            )
            .with_detail("path", candidate.display().to_string())
            .with_detail("error", error.to_string())
        })?;
        self.ensure_inside(&canonical)?;
        Ok(WorkspacePath {
            root: self.clone(),
            relative,
        })
    }

    pub fn resolve_for_create(&self, path: impl AsRef<Path>) -> Result<WorkspacePath, HostError> {
        let relative = normalize_relative(path.as_ref())?;
        let candidate = self.canonical_root.join(&relative);
        let parent = candidate.parent().ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace path has no parent directory",
            )
        })?;
        let canonical_parent = fs::canonicalize(parent).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace path parent does not exist",
            )
            .with_detail("path", parent.display().to_string())
            .with_detail("error", error.to_string())
        })?;
        self.ensure_inside(&canonical_parent)?;
        Ok(WorkspacePath {
            root: self.clone(),
            relative,
        })
    }

    pub fn ensure_inside(&self, canonical_path: &Path) -> Result<(), HostError> {
        if canonical_path.starts_with(&self.canonical_root) {
            Ok(())
        } else {
            Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "workspace path escapes the configured root",
            )
            .with_detail("root", self.canonical_root.display().to_string())
            .with_detail("path", canonical_path.display().to_string()))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspacePath {
    pub root: WorkspaceRoot,
    pub relative: PathBuf,
}

impl WorkspacePath {
    pub fn absolute(&self) -> PathBuf {
        self.root.canonical_root.join(&self.relative)
    }
}

pub fn normalize_relative(path: &Path) -> Result<PathBuf, HostError> {
    let mut relative = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(HostError::new(
                    HostErrorCode::AuthorityDenied,
                    "workspace path traversal is not allowed",
                )
                .with_detail("path", path.display().to_string()));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(HostError::new(
                    HostErrorCode::AuthorityDenied,
                    "absolute workspace paths are not allowed",
                )
                .with_detail("path", path.display().to_string()));
            }
        }
    }
    if relative.as_os_str().is_empty() {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "workspace path must not be empty",
        ));
    }
    Ok(relative)
}
