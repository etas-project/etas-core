use std::{
    io::{Read, Write},
    path::Path,
};

use cap_std::fs::{Dir, OpenOptions};

use super::workspace::{normalize_relative, read_options, workspace_io_error};
use crate::{HostError, HostErrorCode, WorkspacePath, WorkspaceRoot};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceFileMetadata {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemPolicy {
    pub read_roots: Vec<WorkspaceRoot>,
    pub write_roots: Vec<WorkspaceRoot>,
    pub delete_roots: Vec<WorkspaceRoot>,
}

impl FilesystemPolicy {
    pub fn deny_all() -> Self {
        Self {
            read_roots: Vec::new(),
            write_roots: Vec::new(),
            delete_roots: Vec::new(),
        }
    }

    pub fn allow_workspace(root: WorkspaceRoot) -> Self {
        Self {
            read_roots: vec![root.clone()],
            write_roots: vec![root],
            delete_roots: Vec::new(),
        }
    }

    pub fn allow_destructive_workspace(root: WorkspaceRoot) -> Self {
        Self {
            read_roots: vec![root.clone()],
            write_roots: vec![root.clone()],
            delete_roots: vec![root],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemSandbox {
    policy: FilesystemPolicy,
}

impl FilesystemSandbox {
    pub fn new(policy: FilesystemPolicy) -> Self {
        Self { policy }
    }

    pub fn read_file(&self, root: &WorkspaceRoot, path: &Path) -> Result<Vec<u8>, HostError> {
        ensure_root_allowed(
            &self.policy.read_roots,
            root,
            "filesystem read is not allowed",
        )?;
        let relative = normalize_relative(path)?;
        let mut file = root
            .directory()
            .open_with(&relative, &read_options())
            .map_err(workspace_io_error)?;
        if !file.metadata().map_err(workspace_io_error)?.is_file() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace read target is not a file",
            ));
        }
        #[cfg(test)]
        tests::after_resolution();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).map_err(workspace_io_error)?;
        Ok(bytes)
    }

    pub fn atomic_write(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
        bytes: &[u8],
    ) -> Result<WorkspacePath, HostError> {
        ensure_root_allowed(
            &self.policy.write_roots,
            root,
            "filesystem write is not allowed",
        )?;
        let relative = normalize_relative(path)?;
        let (parent, name) = root.open_parent(&relative)?;
        #[cfg(test)]
        tests::after_resolution();
        atomic_write_at(&parent, Path::new(&name), bytes)?;
        Ok(WorkspacePath {
            root: root.clone(),
            relative,
        })
    }

    pub fn create_dir_all(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspacePath, HostError> {
        ensure_root_allowed(
            &self.policy.write_roots,
            root,
            "filesystem write is not allowed",
        )?;
        let relative = normalize_relative(path)?;
        // Each component is created and opened relative to the retained parent.
        let mut parent = root.directory().try_clone().map_err(workspace_io_error)?;
        for component in &relative {
            match parent.create_dir(component) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(workspace_io_error(error)),
            }
            #[cfg(test)]
            tests::after_resolution();
            parent = parent.open_dir(component).map_err(workspace_io_error)?;
        }
        Ok(WorkspacePath {
            root: root.clone(),
            relative,
        })
    }

    pub fn read_dir(&self, root: &WorkspaceRoot, path: &Path) -> Result<Vec<String>, HostError> {
        ensure_root_allowed(
            &self.policy.read_roots,
            root,
            "filesystem read is not allowed",
        )?;
        let directory = root
            .directory()
            .open_dir(normalize_relative(path)?)
            .map_err(workspace_io_error)?;
        #[cfg(test)]
        tests::after_resolution();
        let mut names = directory
            .entries()
            .map_err(workspace_io_error)?
            .map(|entry| {
                entry
                    .map_err(workspace_io_error)?
                    .file_name()
                    .into_string()
                    .map_err(|_| {
                        HostError::new(
                            HostErrorCode::InvalidResponse,
                            "workspace directory entry name is not valid UTF-8",
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        names.sort();
        Ok(names)
    }

    pub fn stat(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspaceFileMetadata, HostError> {
        ensure_root_allowed(
            &self.policy.read_roots,
            root,
            "filesystem read is not allowed",
        )?;
        let metadata = root
            .directory()
            .metadata(normalize_relative(path)?)
            .map_err(workspace_io_error)?;
        #[cfg(test)]
        tests::after_resolution();
        Ok(WorkspaceFileMetadata {
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            len: metadata.len(),
        })
    }

    pub fn delete_file(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspacePath, HostError> {
        ensure_root_allowed(
            &self.policy.delete_roots,
            root,
            "filesystem delete is not allowed",
        )?;
        let relative = normalize_relative(path)?;
        let (parent, name) = root.open_parent(&relative)?;
        let metadata = parent.symlink_metadata(&name).map_err(workspace_io_error)?;
        if !metadata.is_file() && !metadata.is_symlink() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace delete target is not a file",
            ));
        }
        #[cfg(test)]
        tests::after_resolution();
        // Unlink the entry in the authorized parent; never follow its target.
        parent.remove_file(&name).map_err(workspace_io_error)?;
        Ok(WorkspacePath {
            root: root.clone(),
            relative,
        })
    }
}

pub(super) fn ensure_root_allowed(
    roots: &[WorkspaceRoot],
    root: &WorkspaceRoot,
    message: &'static str,
) -> Result<(), HostError> {
    if roots.contains(root) {
        Ok(())
    } else {
        Err(HostError::new(HostErrorCode::AuthorityDenied, message)
            .with_detail("root", root.display_path().display().to_string()))
    }
}

pub(super) fn atomic_write_at(parent: &Dir, name: &Path, bytes: &[u8]) -> Result<(), HostError> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to generate temporary file identity",
        )
        .with_detail("error", error.to_string())
    })?;
    let temp = format!(".etas-tmp-{:032x}", u128::from_le_bytes(random));
    let mut file = parent
        .open_with(&temp, OpenOptions::new().create_new(true).write(true))
        .map_err(workspace_io_error)?;
    let mut published = false;
    let result = (|| {
        #[cfg(test)]
        tests::atomic_stage("pre-write")?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        #[cfg(test)]
        tests::atomic_stage("pre-rename")?;
        parent.rename(&temp, parent, name)?;
        published = true;
        #[cfg(test)]
        tests::atomic_stage("post-rename")?;
        // Linux directory capabilities may be O_PATH descriptors, which cannot
        // be fsynced. Open the bound directory itself, never its display path.
        parent.open(".")?.sync_all()
    })();
    result.map_err(|error| {
        let mut error = workspace_io_error(error).with_detail(
            "publication",
            if published { "committed" } else { "unchanged" },
        );
        if published {
            error = error.with_detail("durability", "uncertain");
        } else if let Err(cleanup) = parent.remove_file(&temp) {
            error = error.with_detail("cleanup_error", cleanup.to_string());
        }
        error
    })
}

#[cfg(test)]
mod tests;
