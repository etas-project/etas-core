use super::WorkspaceRoot;
use crate::{HostError, HostErrorCode};
use std::path::{Component, Path, PathBuf};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspacePath {
    pub root: WorkspaceRoot,
    pub relative: PathBuf,
}

pub fn normalize_relative(path: &Path) -> Result<PathBuf, HostError> {
    let bytes = path.as_os_str().as_encoded_bytes();
    if bytes.contains(&b'\\')
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
    {
        return Err(HostError::new(
            HostErrorCode::AuthorityDenied,
            "drive, UNC and backslash workspace paths are not allowed",
        ));
    }
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
