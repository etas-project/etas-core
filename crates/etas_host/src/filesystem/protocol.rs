use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostRequestId, TraceContext, WorkspacePathRef,
};

#[derive(Clone, Debug, PartialEq)]
pub struct FilesystemRequest {
    pub id: HostRequestId,
    pub operation: FilesystemOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesystemOperation {
    Read {
        path: WorkspacePathRef,
    },
    ReadLimited {
        path: WorkspacePathRef,
        max_bytes: usize,
    },
    Write {
        path: WorkspacePathRef,
        contents: Vec<u8>,
        create_dirs: bool,
    },
    Delete {
        path: WorkspacePathRef,
    },
    ReadDir {
        path: WorkspacePathRef,
    },
    ReadDirLimited {
        path: WorkspacePathRef,
        max_entries: usize,
        max_bytes: usize,
    },
    Stat {
        path: WorkspacePathRef,
    },
    AtomicReplace {
        path: WorkspacePathRef,
        contents: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemResponse {
    pub id: HostRequestId,
    pub result: Result<FilesystemEntry, HostError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundedFileRead {
    Complete(Vec<u8>),
    LimitExceeded { max_bytes: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundedDirectoryList<T = WorkspacePathRef> {
    Complete(Vec<T>),
    LimitExceeded {
        max_entries: usize,
        max_bytes: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesystemEntry {
    Bytes(Vec<u8>),
    LimitedRead(crate::BoundedFileRead),
    LimitedEntries(BoundedDirectoryList),
    Entries(Vec<WorkspacePathRef>),
    Stat(FilesystemStat),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemStat {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
}
