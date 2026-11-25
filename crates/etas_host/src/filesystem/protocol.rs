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
    Open {
        path: WorkspacePathRef,
        mode: FileMode,
    },
    Sync {
        file: FileStreamRef,
    },
    Canonicalize {
        path: WorkspacePathRef,
    },
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
    CreateDir {
        path: WorkspacePathRef,
        recursive: bool,
    },
    Rename {
        source: WorkspacePathRef,
        destination: WorkspacePathRef,
    },
    RemoveFile {
        path: WorkspacePathRef,
    },
    RemoveDir {
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
    FileStream(FileStreamRef),
    Path(WorkspacePathRef),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FileMode {
    Read,
    WriteTruncate,
    Append,
    ReadWrite,
    CreateNew,
}
impl FileMode {
    pub fn readable(self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }
    pub fn writable(self) -> bool {
        self != Self::Read
    }
    pub fn rights(self) -> impl Iterator<Item = &'static str> {
        [
            self.readable().then_some("read"),
            self.writable().then_some("write"),
        ]
        .into_iter()
        .flatten()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStreamRef {
    stream: crate::ByteStreamRef,
}
impl FileStreamRef {
    pub fn issued(handle: crate::StreamHandleRef, path: WorkspacePathRef, mode: FileMode) -> Self {
        Self {
            stream: crate::ByteStreamRef::issued(
                handle,
                crate::ByteStreamOrigin::File {
                    path: path.relative,
                    region: path.region,
                    mode,
                },
            ),
        }
    }
    pub fn handle(&self) -> &crate::StreamHandleRef {
        self.stream.handle()
    }
    pub fn as_byte_stream(&self) -> crate::ByteStreamRef {
        self.stream.clone()
    }
    pub fn path(&self) -> WorkspacePathRef {
        let crate::ByteStreamOrigin::File { path, region, .. } = self.stream.origin() else {
            unreachable!()
        };
        WorkspacePathRef {
            region: region.clone(),
            relative: path.clone(),
        }
    }
    pub fn mode(&self) -> FileMode {
        let crate::ByteStreamOrigin::File { mode, .. } = self.stream.origin() else {
            unreachable!()
        };
        *mode
    }
    pub fn with_read_scope(&self, scope: &crate::StreamReadScope) -> Self {
        Self {
            stream: self.stream.clone().with_read_scope(scope),
        }
    }
}
