use crate::{AuthorityContext, Budget, HostError, HostRequestId, TraceContext, WorkspacePath};

#[derive(Clone, Debug, PartialEq)]
pub struct FilesystemRequest {
    pub id: HostRequestId,
    pub operation: FilesystemOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilesystemOperation {
    Read {
        path: WorkspacePath,
    },
    Write {
        path: WorkspacePath,
        contents: Vec<u8>,
        create_dirs: bool,
    },
    Delete {
        path: WorkspacePath,
    },
    ReadDir {
        path: WorkspacePath,
    },
    Stat {
        path: WorkspacePath,
    },
    AtomicReplace {
        path: WorkspacePath,
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
    Entries(Vec<String>),
    Stat(FilesystemStat),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilesystemStat {
    pub is_file: bool,
    pub is_dir: bool,
    pub len: u64,
}
