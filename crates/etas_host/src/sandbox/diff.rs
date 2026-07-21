use std::path::PathBuf;

use crate::{WorkspaceRoot, WorkspaceSnapshotEntry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceDiff {
    pub root: WorkspaceRoot,
    pub entries: Vec<WorkspaceDiffEntry>,
}

impl WorkspaceDiff {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceDiffEntry {
    pub path: PathBuf,
    pub kind: WorkspaceDiffKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceDiffKind {
    Added {
        after: WorkspaceSnapshotEntry,
    },
    Deleted {
        before: WorkspaceSnapshotEntry,
    },
    Modified {
        before: WorkspaceSnapshotEntry,
        after: WorkspaceSnapshotEntry,
    },
}
