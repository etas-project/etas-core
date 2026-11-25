use super::{FilesystemSandbox, ensure_root_allowed};
use crate::sandbox::workspace::{normalize_directory_relative, workspace_io_error};
use crate::{HostError, WorkspacePath, WorkspaceRoot};
use std::path::Path;

impl FilesystemSandbox {
    pub fn create_dir(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspacePath, HostError> {
        ensure_root_allowed(
            &self.policy.write_roots,
            root,
            "filesystem write is not allowed",
        )?;
        let relative = normalize_directory_relative(path)?;
        if relative != Path::new(".") {
            let (parent, name) = root.open_parent(&relative)?;
            match parent.create_dir(&name) {
                Ok(()) => (),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    parent.open_dir(&name).map_err(workspace_io_error)?;
                }
                Err(error) => return Err(workspace_io_error(error)),
            }
        }
        Ok(WorkspacePath {
            root: root.clone(),
            relative,
        })
    }

    pub fn rename(
        &self,
        root: &WorkspaceRoot,
        source: &Path,
        destination: &Path,
    ) -> Result<(), HostError> {
        ensure_root_allowed(
            &self.policy.write_roots,
            root,
            "filesystem write is not allowed",
        )?;
        let (source_parent, source_name) = root.open_parent(source)?;
        let (destination_parent, destination_name) = root.open_parent(destination)?;
        rename_no_replace(
            &source_parent,
            &source_name,
            &destination_parent,
            &destination_name,
        )
        .map_err(workspace_io_error)
    }

    pub fn remove_dir(&self, root: &WorkspaceRoot, path: &Path) -> Result<(), HostError> {
        ensure_root_allowed(
            &self.policy.delete_roots,
            root,
            "filesystem delete is not allowed",
        )?;
        let (parent, name) = root.open_parent(path)?;
        parent.remove_dir(&name).map_err(workspace_io_error)
    }
}

#[cfg(target_os = "linux")]
fn rename_no_replace(
    source_parent: &cap_std::fs::Dir,
    source: &std::ffi::OsStr,
    destination_parent: &cap_std::fs::Dir,
    destination: &std::ffi::OsStr,
) -> std::io::Result<()> {
    use std::{
        ffi::CString,
        os::{fd::AsRawFd, unix::ffi::OsStrExt},
    };
    let source = CString::new(source.as_bytes())?;
    let destination = CString::new(destination.as_bytes())?;
    // Both parents are retained directory capabilities and names are single
    // components. RENAME_NOREPLACE enforces the contract in the OS operation.
    let result = unsafe {
        libc::renameat2(
            source_parent.as_raw_fd(),
            source.as_ptr(),
            destination_parent.as_raw_fd(),
            destination.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(target_os = "linux"))]
fn rename_no_replace(
    _source_parent: &cap_std::fs::Dir,
    _source: &std::ffi::OsStr,
    _destination_parent: &cap_std::fs::Dir,
    _destination: &std::ffi::OsStr,
) -> std::io::Result<()> {
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}
