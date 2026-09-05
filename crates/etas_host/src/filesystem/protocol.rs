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
pub enum FilesystemEntry {
    Bytes(Vec<u8>),
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
