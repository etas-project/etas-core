use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

use cap_std::fs::{Dir, OpenOptions};

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

#[derive(Clone, Debug)]
pub struct WorkspaceRoot {
    pub canonical_root: PathBuf,
    directory: Arc<Dir>,
    identity: Arc<same_file::Handle>,
}

impl PartialEq for WorkspaceRoot {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}

impl Eq for WorkspaceRoot {}

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
        let directory = Dir::open_ambient_dir(&canonical_root, cap_std::ambient_authority())
            .map_err(workspace_io_error)?;
        let identity = same_file::Handle::from_file(
            directory
                .try_clone()
                .map_err(workspace_io_error)?
                .into_std_file(),
        )
        .map_err(workspace_io_error)?;
        Ok(Self {
            canonical_root,
            directory: Arc::new(directory),
            identity: Arc::new(identity),
        })
    }

    pub fn resolve_existing(&self, path: impl AsRef<Path>) -> Result<WorkspacePath, HostError> {
        let relative = normalize_relative(path.as_ref())?;
        self.directory
            .metadata(&relative)
            .map_err(workspace_io_error)?;
        Ok(WorkspacePath {
            root: self.clone(),
            relative,
        })
    }

    pub fn resolve_for_create(&self, path: impl AsRef<Path>) -> Result<WorkspacePath, HostError> {
        let relative = normalize_relative(path.as_ref())?;
        self.open_parent(&relative)?;
        Ok(WorkspacePath {
            root: self.clone(),
            relative,
        })
    }

    pub(crate) fn directory(&self) -> &Dir {
        &self.directory
    }

    pub(crate) fn open_parent(&self, path: &Path) -> Result<(Dir, std::ffi::OsString), HostError> {
        let relative = normalize_relative(path)?;
        let name = relative
            .file_name()
            .ok_or_else(|| {
                HostError::new(
                    HostErrorCode::InvalidRequest,
                    "workspace path has no file name",
                )
            })?
            .to_owned();
        let parent = relative
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        Ok((
            self.directory
                .open_dir(parent)
                .map_err(workspace_io_error)?,
            name,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspacePath {
    pub root: WorkspaceRoot,
    pub relative: PathBuf,
}

pub(crate) fn workspace_io_error(error: std::io::Error) -> HostError {
    let code = match error.kind() {
        std::io::ErrorKind::PermissionDenied => HostErrorCode::AuthorityDenied,
        std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidInput => {
            HostErrorCode::InvalidRequest
        }
        _ => HostErrorCode::ProviderUnavailable,
    };
    HostError::new(code, "workspace capability operation failed")
        .with_detail("error", error.to_string())
}

pub(super) fn read_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use cap_std::fs::OpenOptionsExt;
        // Opening a raced-in FIFO must not block before its file kind is checked.
        options.custom_flags(libc::O_NONBLOCK);
    }
    options
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
