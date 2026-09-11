use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
};

use super::{
    filesystem::atomic_write_at,
    workspace::{read_options, workspace_io_error},
};
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::fs::Dir;

use crate::{
    HostError, HostErrorCode, WorkspaceDiff, WorkspaceDiffEntry, WorkspaceDiffKind, WorkspaceRoot,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    root: WorkspaceRoot,
    entries: BTreeMap<PathBuf, WorkspaceSnapshotEntry>,
}

impl WorkspaceSnapshot {
    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub fn entries(&self) -> &BTreeMap<PathBuf, WorkspaceSnapshotEntry> {
        &self.entries
    }

    pub fn capture(root: WorkspaceRoot) -> Result<Self, HostError> {
        let mut entries = BTreeMap::new();
        capture_dir(root.directory(), Path::new(""), &mut entries)?;
        Ok(Self { root, entries })
    }

    pub fn diff_current(&self) -> Result<WorkspaceDiff, HostError> {
        let current = Self::capture(self.root.clone())?;
        let mut paths = BTreeSet::new();
        paths.extend(self.entries.keys().cloned());
        paths.extend(current.entries.keys().cloned());

        let mut entries = Vec::new();
        for path in paths {
            match (self.entries.get(&path), current.entries.get(&path)) {
                (None, Some(after)) => entries.push(WorkspaceDiffEntry {
                    path,
                    kind: WorkspaceDiffKind::Added {
                        after: after.clone(),
                    },
                }),
                (Some(before), None) => entries.push(WorkspaceDiffEntry {
                    path,
                    kind: WorkspaceDiffKind::Deleted {
                        before: before.clone(),
                    },
                }),
                (Some(before), Some(after)) if before != after => {
                    entries.push(WorkspaceDiffEntry {
                        path,
                        kind: WorkspaceDiffKind::Modified {
                            before: before.clone(),
                            after: after.clone(),
                        },
                    })
                }
                _ => {}
            }
        }
        Ok(WorkspaceDiff {
            root: self.root.clone(),
            entries,
        })
    }

    fn rollback_staged(&self) -> Result<WorkspaceDiff, HostError> {
        let diff = self.diff_current()?;
        // Remove children before their directories; restore directories before
        // their children. Every operation stays relative to a retained parent.
        for entry in diff
            .entries
            .iter()
            .rev()
            .filter(|entry| matches!(entry.kind, WorkspaceDiffKind::Added { .. }))
        {
            let (parent, name) = self.root.open_parent(&entry.path)?;
            #[cfg(test)]
            tests::after_parent_resolution();
            remove_existing(&parent, Path::new(&name))?;
        }
        for entry in &diff.entries {
            if let WorkspaceDiffKind::Deleted { before }
            | WorkspaceDiffKind::Modified { before, .. } = &entry.kind
            {
                let (parent, name) = self.root.open_parent(&entry.path)?;
                #[cfg(test)]
                tests::after_parent_resolution();
                restore_snapshot_entry(&parent, Path::new(&name), before)?;
            }
        }
        Ok(diff)
    }
}

/// A separately provisioned, initially empty staging directory. Applications
/// control its lifetime and exclude competing writers; no live tree is copied
/// or automatically published to a destination.
#[derive(Debug)]
pub struct WorkspaceStage {
    root: WorkspaceRoot,
}

impl WorkspaceStage {
    pub fn create(parent: &WorkspaceRoot) -> Result<Self, HostError> {
        let mut identity = [0u8; 16];
        getrandom::fill(&mut identity).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "cannot create staging identity",
            )
            .with_detail("error", error.to_string())
        })?;
        let name = format!(".etas-stage-{:032x}", u128::from_le_bytes(identity));
        let mut builder = cap_std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use cap_std::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        parent
            .directory()
            .create_dir_with(&name, &builder)
            .map_err(workspace_io_error)?;
        let root = parent.open_child(&name)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &WorkspaceRoot {
        &self.root
    }

    pub fn snapshot(&self) -> Result<StagedWorkspaceSnapshot, HostError> {
        Ok(StagedWorkspaceSnapshot {
            snapshot: WorkspaceSnapshot::capture(self.root.clone())?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StagedWorkspaceSnapshot {
    snapshot: WorkspaceSnapshot,
}

impl StagedWorkspaceSnapshot {
    pub fn root(&self) -> &WorkspaceRoot {
        self.snapshot.root()
    }

    pub fn diff_current(&self) -> Result<WorkspaceDiff, HostError> {
        self.snapshot.diff_current()
    }

    pub fn rollback(&self) -> Result<WorkspaceDiff, HostError> {
        self.snapshot.rollback_staged()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceSnapshotEntry {
    Directory,
    File { bytes: Vec<u8> },
    Symlink { target: PathBuf },
}

fn capture_dir(
    directory: &Dir,
    relative: &Path,
    entries: &mut BTreeMap<PathBuf, WorkspaceSnapshotEntry>,
) -> Result<(), HostError> {
    for entry in directory.entries().map_err(workspace_io_error)? {
        let entry = entry.map_err(workspace_io_error)?;
        let name = entry.file_name();
        let entry_relative = relative.join(&name);
        let file_type = entry.file_type().map_err(workspace_io_error)?;
        #[cfg(test)]
        tests::after_resolution();
        if file_type.is_symlink() {
            let target = directory
                .read_link_contents(&name)
                .map_err(workspace_io_error)?;
            entries.insert(entry_relative, WorkspaceSnapshotEntry::Symlink { target });
        } else if file_type.is_dir() {
            let child = directory
                .open_dir_nofollow(&name)
                .map_err(workspace_io_error)?;
            entries.insert(entry_relative.clone(), WorkspaceSnapshotEntry::Directory);
            capture_dir(&child, &entry_relative, entries)?;
        } else if file_type.is_file() {
            let mut file = directory
                .open_with(&name, read_options().follow(FollowSymlinks::No))
                .map_err(workspace_io_error)?;
            if !file.metadata().map_err(workspace_io_error)?.is_file() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "workspace snapshot file changed kind during capture",
                ));
            }
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).map_err(workspace_io_error)?;
            entries.insert(entry_relative, WorkspaceSnapshotEntry::File { bytes });
        } else {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace snapshot contains an unsupported file kind",
            ));
        }
    }
    Ok(())
}

fn restore_snapshot_entry(
    parent: &Dir,
    name: &Path,
    entry: &WorkspaceSnapshotEntry,
) -> Result<(), HostError> {
    match parent.symlink_metadata(name) {
        Ok(_) => remove_existing(parent, name)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(workspace_io_error(error)),
    }
    match entry {
        WorkspaceSnapshotEntry::Directory => parent.create_dir(name).map_err(workspace_io_error),
        WorkspaceSnapshotEntry::File { bytes } => atomic_write_at(parent, name, bytes),
        WorkspaceSnapshotEntry::Symlink { target } => {
            restore_symlink(parent, name, target).map_err(workspace_io_error)
        }
    }
}

fn remove_existing(parent: &Dir, name: &Path) -> Result<(), HostError> {
    let metadata = parent.symlink_metadata(name).map_err(workspace_io_error)?;
    if metadata.is_dir() {
        parent.remove_dir_all(name).map_err(workspace_io_error)
    } else {
        parent.remove_file(name).map_err(workspace_io_error)
    }
}

#[cfg(unix)]
fn restore_symlink(parent: &Dir, name: &Path, target: &Path) -> Result<(), std::io::Error> {
    parent.symlink_contents(target, name)
}

#[cfg(windows)]
fn restore_symlink(parent: &Dir, name: &Path, target: &Path) -> Result<(), std::io::Error> {
    parent.symlink_file(target, name)
}

#[cfg(test)]
mod tests;
