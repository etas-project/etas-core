use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{HostError, HostErrorCode};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceRegionId(String);

impl WorkspaceRegionId {
    pub fn new(identity: impl Into<String>) -> Result<Self, HostError> {
        let identity = identity.into();
        if identity.is_empty() || identity.split('.').any(|segment| !is_identifier(segment)) {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace region identity must be a non-empty canonical type path",
            )
            .with_detail("region", identity));
        }
        Ok(Self(identity))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_identifier(segment: &str) -> bool {
    let mut characters = segment.chars();
    matches!(characters.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspacePathRef {
    pub region: WorkspaceRegionId,
    pub relative: PathBuf,
}

impl WorkspacePathRef {
    pub fn new(region: WorkspaceRegionId, relative: impl AsRef<Path>) -> Result<Self, HostError> {
        Ok(Self {
            region,
            relative: normalize_relative(relative.as_ref())?,
        })
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_region_identity_uses_canonical_language_identifiers() {
        assert!(WorkspaceRegionId::new("app.workspace.ProjectRoot").is_ok());
        assert!(WorkspaceRegionId::new("app._private.Root2").is_ok());

        for invalid in ["", ".app.Root", "app..Root", "app.1Root", "app.Root-name"] {
            assert!(
                WorkspaceRegionId::new(invalid).is_err(),
                "`{invalid}` must not be accepted as a canonical region identity"
            );
        }
    }

    #[test]
    fn workspace_path_ref_rejects_absolute_and_parent_paths() {
        let region = WorkspaceRegionId::new("app.workspace.ProjectRoot").expect("valid region");
        assert!(WorkspacePathRef::new(region.clone(), "src/main.es").is_ok());
        assert!(WorkspacePathRef::new(region.clone(), "../secret").is_err());
        assert!(WorkspacePathRef::new(region, "/tmp/secret").is_err());
    }
}
