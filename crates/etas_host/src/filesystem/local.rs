use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

use crate::{
    ActionInstance, FilesystemClient, FilesystemEntry, FilesystemOperation, FilesystemRequest,
    FilesystemResponse, FilesystemStat, HostError, HostErrorCode, HostValue, SandboxBroker,
    WorkspacePath, WorkspacePathRef, WorkspaceRegionId, WorkspaceRoot,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorkspaceRegionRegistry {
    roots: BTreeMap<WorkspaceRegionId, WorkspaceRoot>,
}

impl WorkspaceRegionRegistry {
    pub fn insert(
        &mut self,
        region: WorkspaceRegionId,
        root: WorkspaceRoot,
    ) -> Result<(), HostError> {
        if self.roots.insert(region.clone(), root).is_some() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "workspace region is configured more than once",
            )
            .with_detail("region", region.as_str()));
        }
        Ok(())
    }

    fn resolve(&self, path: &WorkspacePathRef, create: bool) -> Result<WorkspacePath, HostError> {
        let root = self.roots.get(&path.region).ok_or_else(|| {
            HostError::new(
                HostErrorCode::AuthorityDenied,
                "workspace region is not configured",
            )
            .with_detail("region", path.region.as_str())
        })?;
        if create {
            root.resolve_for_create(&path.relative)
        } else {
            root.resolve_existing(&path.relative)
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LocalFilesystemClient {
    regions: Arc<WorkspaceRegionRegistry>,
}

impl LocalFilesystemClient {
    pub fn new(regions: WorkspaceRegionRegistry) -> Self {
        Self {
            regions: Arc::new(regions),
        }
    }
}

impl FilesystemClient for LocalFilesystemClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<FilesystemResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: FilesystemRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            let id = request.id;
            let result = execute_local_filesystem(&self.regions, request);
            Ok(FilesystemResponse { id, result })
        })
    }
}

fn execute_local_filesystem(
    regions: &WorkspaceRegionRegistry,
    request: FilesystemRequest,
) -> Result<FilesystemEntry, HostError> {
    require_filesystem_authority(&request)?;
    let broker = SandboxBroker::new(request.authority.sandbox);
    match request.operation {
        FilesystemOperation::Read { path } => {
            let path = regions.resolve(&path, false)?;
            broker
                .read_file(&path.root, &path.relative)
                .map(FilesystemEntry::Bytes)
        }
        FilesystemOperation::Write {
            path,
            contents,
            create_dirs,
        } => {
            let path = regions.resolve(&path, true)?;
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
            let path = regions.resolve(&path, false)?;
            broker.delete_file(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::ReadDir { path } => {
            let region = path.region.clone();
            let parent = path.relative.clone();
            let path = regions.resolve(&path, false)?;
            if path.relative.as_os_str().is_empty() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "filesystem read_dir path must not be empty",
                ));
            }
            let entries = broker
                .read_dir(&path.root, &path.relative)?
                .into_iter()
                .map(|entry| WorkspacePathRef::new(region.clone(), parent.join(entry)))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(FilesystemEntry::Entries(entries))
        }
        FilesystemOperation::Stat { path } => {
            let path = regions.resolve(&path, false)?;
            let metadata = broker.stat(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Stat(FilesystemStat {
                is_file: metadata.is_file,
                is_dir: metadata.is_dir,
                len: metadata.len,
            }))
        }
        FilesystemOperation::AtomicReplace { path, contents } => {
            let path = regions.resolve(&path, true)?;
            broker.atomic_write(&path.root, &path.relative, &contents)?;
            Ok(FilesystemEntry::Unit)
        }
    }
}

fn require_filesystem_authority(request: &FilesystemRequest) -> Result<(), HostError> {
    let (action, path) = match &request.operation {
        FilesystemOperation::Read { path } => ("read", path),
        FilesystemOperation::Write { path, .. } => ("write", path),
        FilesystemOperation::Delete { path } => ("delete", path),
        FilesystemOperation::ReadDir { path } => ("list", path),
        FilesystemOperation::Stat { path } => ("stat", path),
        FilesystemOperation::AtomicReplace { path, .. } => ("atomic_replace", path),
    };
    let instance = ActionInstance::new(
        "Fs",
        action,
        vec![HostValue::String(path.region.as_str().to_owned())],
    );
    if request.authority.allows(&instance) {
        return Ok(());
    }
    Err(HostError::new(
        HostErrorCode::AuthorityDenied,
        "filesystem request is missing its region-scoped action grant",
    )
    .with_detail("action", format!("Fs.{action}"))
    .with_detail("region", path.region.as_str()))
}
