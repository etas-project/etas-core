use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    HostError, HostErrorCode, WorkspaceDiff, WorkspaceDiffEntry, WorkspaceDiffKind, WorkspaceRoot,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    pub root: WorkspaceRoot,
    pub entries: BTreeMap<PathBuf, WorkspaceSnapshotEntry>,
}

impl WorkspaceSnapshot {
    pub fn capture(root: WorkspaceRoot) -> Result<Self, HostError> {
        let mut entries = BTreeMap::new();
        capture_dir(&root, Path::new(""), &mut entries)?;
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

    pub fn rollback(&self) -> Result<WorkspaceDiff, HostError> {
        let diff = self.diff_current()?;
        for entry in diff.entries.iter().rev() {
            let absolute = self.root.canonical_root.join(&entry.path);
            match &entry.kind {
                WorkspaceDiffKind::Added { after } => remove_snapshot_entry(&absolute, after)?,
                WorkspaceDiffKind::Deleted { before }
                | WorkspaceDiffKind::Modified { before, .. } => {
                    restore_snapshot_entry(&absolute, before)?;
                }
            }
        }
        Ok(diff)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceSnapshotEntry {
    Directory,
    File { bytes: Vec<u8> },
    Symlink { target: PathBuf },
}

fn capture_dir(
    root: &WorkspaceRoot,
    relative: &Path,
    entries: &mut BTreeMap<PathBuf, WorkspaceSnapshotEntry>,
) -> Result<(), HostError> {
    let absolute = root.canonical_root.join(relative);
    for entry in fs::read_dir(&absolute).map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to read workspace directory for snapshot",
        )
        .with_detail("path", absolute.display().to_string())
        .with_detail("error", error.to_string())
    })? {
        let entry = entry.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to inspect workspace directory entry",
            )
            .with_detail("error", error.to_string())
        })?;
        let file_name = entry.file_name();
        let entry_relative = relative.join(file_name);
        let file_type = entry.file_type().map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to read workspace entry type",
            )
            .with_detail("path", entry.path().display().to_string())
            .with_detail("error", error.to_string())
        })?;
        if file_type.is_symlink() {
            let target = fs::read_link(entry.path()).map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to read workspace symlink",
                )
                .with_detail("path", entry.path().display().to_string())
                .with_detail("error", error.to_string())
            })?;
            entries.insert(entry_relative, WorkspaceSnapshotEntry::Symlink { target });
        } else if file_type.is_dir() {
            entries.insert(entry_relative.clone(), WorkspaceSnapshotEntry::Directory);
            capture_dir(root, &entry_relative, entries)?;
        } else if file_type.is_file() {
            let bytes = fs::read(entry.path()).map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to read workspace file for snapshot",
                )
                .with_detail("path", entry.path().display().to_string())
                .with_detail("error", error.to_string())
            })?;
            entries.insert(entry_relative, WorkspaceSnapshotEntry::File { bytes });
        }
    }
    Ok(())
}

fn remove_snapshot_entry(path: &Path, entry: &WorkspaceSnapshotEntry) -> Result<(), HostError> {
    match entry {
        WorkspaceSnapshotEntry::Directory => fs::remove_dir_all(path),
        WorkspaceSnapshotEntry::File { .. } | WorkspaceSnapshotEntry::Symlink { .. } => {
            fs::remove_file(path)
        }
    }
    .map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to remove workspace rollback entry",
        )
        .with_detail("path", path.display().to_string())
        .with_detail("error", error.to_string())
    })
}

fn restore_snapshot_entry(path: &Path, entry: &WorkspaceSnapshotEntry) -> Result<(), HostError> {
    if path.exists() {
        remove_existing(path)?;
    }
    match entry {
        WorkspaceSnapshotEntry::Directory => fs::create_dir_all(path),
        WorkspaceSnapshotEntry::File { bytes } => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(rollback_error)?;
            }
            fs::write(path, bytes)
        }
        WorkspaceSnapshotEntry::Symlink { target } => restore_symlink(path, target),
    }
    .map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to restore workspace rollback entry",
        )
        .with_detail("path", path.display().to_string())
        .with_detail("error", error.to_string())
    })
}

fn remove_existing(path: &Path) -> Result<(), HostError> {
    let metadata = fs::symlink_metadata(path).map_err(rollback_error)?;
    let result = if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    result.map_err(rollback_error)
}

#[cfg(unix)]
fn restore_symlink(path: &Path, target: &Path) -> Result<(), std::io::Error> {
    std::os::unix::fs::symlink(target, path)
}

#[cfg(windows)]
fn restore_symlink(path: &Path, target: &Path) -> Result<(), std::io::Error> {
    std::os::windows::fs::symlink_file(target, path)
}

fn rollback_error(error: std::io::Error) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "workspace rollback filesystem operation failed",
    )
    .with_detail("error", error.to_string())
}
