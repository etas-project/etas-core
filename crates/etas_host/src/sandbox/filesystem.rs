use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

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
        self.ensure_read_root(root)?;
        let workspace_path = root.resolve_existing(path)?;
        let absolute = workspace_path.absolute();
        if !absolute.is_file() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace read target is not a file",
            )
            .with_detail("path", absolute.display().to_string()));
        }
        fs::read(&absolute).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to read workspace file",
            )
            .with_detail("path", absolute.display().to_string())
            .with_detail("error", error.to_string())
        })
    }

    pub fn atomic_write(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
        bytes: &[u8],
    ) -> Result<WorkspacePath, HostError> {
        self.ensure_write_root(root)?;
        let workspace_path = root.resolve_for_create(path)?;
        let absolute = workspace_path.absolute();
        let parent = absolute.parent().ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace write target has no parent directory",
            )
        })?;
        root.ensure_inside(&fs::canonicalize(parent).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace write parent does not exist",
            )
            .with_detail("path", parent.display().to_string())
            .with_detail("error", error.to_string())
        })?)?;

        let temp_path = temp_write_path(parent, &absolute)?;
        let write_result = write_temp_file(&temp_path, bytes)
            .and_then(|_| fs::rename(&temp_path, &absolute).map_err(fs_error));
        match write_result {
            Ok(()) => Ok(workspace_path),
            Err(error) => {
                let _ = fs::remove_file(&temp_path);
                Err(error)
            }
        }
    }

    pub fn create_dir_all(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspacePath, HostError> {
        self.ensure_write_root(root)?;
        let workspace_path = root.resolve_for_create(path)?;
        let absolute = workspace_path.absolute();
        fs::create_dir_all(&absolute).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to create workspace directory",
            )
            .with_detail("path", absolute.display().to_string())
            .with_detail("error", error.to_string())
        })?;
        root.ensure_inside(&fs::canonicalize(&absolute).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "created workspace directory could not be canonicalized",
            )
            .with_detail("path", absolute.display().to_string())
            .with_detail("error", error.to_string())
        })?)?;
        Ok(workspace_path)
    }

    pub fn read_dir(&self, root: &WorkspaceRoot, path: &Path) -> Result<Vec<String>, HostError> {
        self.ensure_read_root(root)?;
        let workspace_path = root.resolve_existing(path)?;
        let absolute = workspace_path.absolute();
        if !absolute.is_dir() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace read_dir target is not a directory",
            )
            .with_detail("path", absolute.display().to_string()));
        }
        let mut entries = fs::read_dir(&absolute)
            .map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to read workspace directory",
                )
                .with_detail("path", absolute.display().to_string())
                .with_detail("error", error.to_string())
            })?
            .map(|entry| {
                let entry = entry.map_err(|error| {
                    HostError::new(
                        HostErrorCode::ProviderUnavailable,
                        "failed to read workspace directory entry",
                    )
                    .with_detail("path", absolute.display().to_string())
                    .with_detail("error", error.to_string())
                })?;
                entry.file_name().into_string().map_err(|_| {
                    HostError::new(
                        HostErrorCode::InvalidResponse,
                        "workspace directory entry name is not valid UTF-8",
                    )
                    .with_detail("path", absolute.display().to_string())
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        Ok(entries)
    }

    pub fn stat(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspaceFileMetadata, HostError> {
        self.ensure_read_root(root)?;
        let workspace_path = root.resolve_existing(path)?;
        let absolute = workspace_path.absolute();
        let metadata = fs::metadata(&absolute).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to stat workspace path",
            )
            .with_detail("path", absolute.display().to_string())
            .with_detail("error", error.to_string())
        })?;
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
        self.ensure_delete_root(root)?;
        let workspace_path = root.resolve_existing(path)?;
        let absolute = workspace_path.absolute();
        if !absolute.is_file() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace delete target is not a file",
            )
            .with_detail("path", absolute.display().to_string()));
        }
        fs::remove_file(&absolute).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to delete workspace file",
            )
            .with_detail("path", absolute.display().to_string())
            .with_detail("error", error.to_string())
        })?;
        Ok(workspace_path)
    }

    fn ensure_read_root(&self, root: &WorkspaceRoot) -> Result<(), HostError> {
        ensure_root_allowed(
            &self.policy.read_roots,
            root,
            "filesystem read is not allowed",
        )
    }

    fn ensure_write_root(&self, root: &WorkspaceRoot) -> Result<(), HostError> {
        ensure_root_allowed(
            &self.policy.write_roots,
            root,
            "filesystem write is not allowed",
        )
    }

    fn ensure_delete_root(&self, root: &WorkspaceRoot) -> Result<(), HostError> {
        ensure_root_allowed(
            &self.policy.delete_roots,
            root,
            "filesystem delete is not allowed",
        )
    }
}

fn ensure_root_allowed(
    allowed_roots: &[WorkspaceRoot],
    root: &WorkspaceRoot,
    message: &'static str,
) -> Result<(), HostError> {
    if allowed_roots.contains(root) {
        Ok(())
    } else {
        Err(HostError::new(HostErrorCode::AuthorityDenied, message)
            .with_detail("root", root.canonical_root.display().to_string()))
    }
}

fn temp_write_path(parent: &Path, final_path: &Path) -> Result<PathBuf, HostError> {
    let final_name = final_path.file_name().ok_or_else(|| {
        HostError::new(
            HostErrorCode::InvalidRequest,
            "workspace write target has no file name",
        )
    })?;
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
    Ok(parent.join(format!(
        ".{}.etas-tmp-{nanos}",
        final_name.to_string_lossy()
    )))
}

fn write_temp_file(path: &Path, bytes: &[u8]) -> Result<(), HostError> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to create temporary workspace file",
            )
            .with_detail("path", path.display().to_string())
            .with_detail("error", error.to_string())
        })?;
    file.write_all(bytes).map_err(fs_error)?;
    file.sync_all().map_err(fs_error)
}

fn fs_error(error: std::io::Error) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "filesystem operation failed",
    )
    .with_detail("error", error.to_string())
}
