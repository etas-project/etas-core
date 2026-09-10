use super::{WorkspacePath, normalize_relative, workspace_io_error};
use crate::{HostError, HostErrorCode};
use cap_std::fs::Dir;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct WorkspaceRoot {
    binding: Arc<RootBinding>,
}

#[derive(Debug)]
struct RootBinding {
    directory: Dir,
    display_path: PathBuf,
}

impl PartialEq for WorkspaceRoot {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.binding, &other.binding)
    }
}
impl Eq for WorkspaceRoot {}

impl WorkspaceRoot {
    /// Opens a fresh authorization binding. Display paths never establish equality.
    pub fn new(root: impl AsRef<Path>) -> Result<Self, HostError> {
        let display_path = std::path::absolute(root.as_ref()).map_err(workspace_io_error)?;
        let directory = Dir::open_ambient_dir(root.as_ref(), cap_std::ambient_authority())
            .map_err(workspace_io_error)?;
        Ok(Self {
            binding: Arc::new(RootBinding {
                directory,
                display_path,
            }),
        })
    }

    /// Open a candidate relative to an already trusted directory, without ambient lookup.
    pub fn open_child(&self, path: impl AsRef<Path>) -> Result<Self, HostError> {
        let relative = normalize_relative(path.as_ref())?;
        let directory = self
            .directory()
            .open_dir(&relative)
            .map_err(workspace_io_error)?;
        Ok(Self {
            binding: Arc::new(RootBinding {
                directory,
                display_path: self.display_path().join(relative),
            }),
        })
    }

    pub fn display_path(&self) -> &Path {
        &self.binding.display_path
    }

    pub(crate) fn directory(&self) -> &Dir {
        &self.binding.directory
    }

    pub fn resolve_existing(&self, path: impl AsRef<Path>) -> Result<WorkspacePath, HostError> {
        let relative = normalize_relative(path.as_ref())?;
        self.directory()
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
            self.directory()
                .open_dir(parent)
                .map_err(workspace_io_error)?,
            name,
        ))
    }
}
