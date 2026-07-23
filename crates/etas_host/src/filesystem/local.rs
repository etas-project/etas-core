use std::{future::Future, pin::Pin};

use crate::{
    FilesystemClient, FilesystemEntry, FilesystemOperation, FilesystemRequest, FilesystemResponse,
    FilesystemStat, HostError, HostErrorCode, SandboxBroker,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalFilesystemClient;

impl LocalFilesystemClient {
    pub fn new() -> Self {
        Self
    }
}

impl FilesystemClient for LocalFilesystemClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<FilesystemResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: FilesystemRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            let id = request.id;
            let result = execute_local_filesystem(request);
            Ok(FilesystemResponse { id, result })
        })
    }
}

fn execute_local_filesystem(request: FilesystemRequest) -> Result<FilesystemEntry, HostError> {
    let broker = SandboxBroker::new(request.authority.sandbox);
    match request.operation {
        FilesystemOperation::Read { path } => broker
            .read_file(&path.root, &path.relative)
            .map(FilesystemEntry::Bytes),
        FilesystemOperation::Write {
            path,
            contents,
            create_dirs,
        } => {
            if create_dirs
                && let Some(parent) = path.relative.parent()
                && !parent.as_os_str().is_empty()
            {
                broker.create_dir_all(&path.root, parent)?;
            }
            broker.atomic_write(&path.root, &path.relative, &contents)?;
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::Delete { path } => {
            broker.delete_file(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::ReadDir { path } => {
            if path.relative.as_os_str().is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "filesystem read_dir path must not be empty",
                ));
            }
            broker
                .read_dir(&path.root, &path.relative)
                .map(FilesystemEntry::Entries)
        }
        FilesystemOperation::Stat { path } => {
            let metadata = broker.stat(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Stat(FilesystemStat {
                is_file: metadata.is_file,
                is_dir: metadata.is_dir,
                len: metadata.len,
            }))
        }
        FilesystemOperation::AtomicReplace { path, contents } => {
            broker.atomic_write(&path.root, &path.relative, &contents)?;
            Ok(FilesystemEntry::Unit)
        }
    }
}
