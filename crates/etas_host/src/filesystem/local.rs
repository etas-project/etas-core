use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    ActionInstance, FilesystemClient, FilesystemEntry, FilesystemOperation, FilesystemRequest,
    FilesystemResponse, FilesystemStat, HostError, HostErrorCode, HostValue, SandboxBroker,
    WorkspacePathRef, WorkspaceRegionRegistry,
};

#[derive(Clone, Default)]
pub struct LocalFilesystemClient {
    regions: Arc<WorkspaceRegionRegistry>,
    streams: crate::ByteStreamStore,
}

impl LocalFilesystemClient {
    pub async fn execute_scoped(
        &self,
        request: FilesystemRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<FilesystemResponse, HostError> {
        let client = self.clone();
        if matches!(
            request.operation,
            FilesystemOperation::Open { .. } | FilesystemOperation::Sync { .. }
        ) {
            return operation
                .supervise(move |context| async move {
                    client.execute_managed(request, Some(&context)).await
                })
                .await;
        }
        operation
            .run_blocking(request.budget.clone(), move |operation| {
                let id = request.id;
                let result = execute_local_filesystem(&client.regions, request, Some(&operation));
                Ok(FilesystemResponse { id, result })
            })
            .await
    }
    pub fn with_streams(mut self, streams: crate::ByteStreamStore) -> Self {
        self.streams = streams;
        self
    }

    async fn execute_managed(
        &self,
        request: FilesystemRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<FilesystemResponse, HostError> {
        require_filesystem_authority(&request)?;
        let id = request.id;
        let result = match request.operation.clone() {
            FilesystemOperation::Open { path, mode } => {
                let client = self.clone();
                let requested_path = path.clone();
                let budget = request.budget.clone();
                let open = move || -> Result<OpenedFile, HostError> {
                    let bound = client.regions.bind(&requested_path)?;
                    let file = SandboxBroker::new(request.authority.sandbox).open_file(
                        &bound.root,
                        &bound.relative,
                        mode,
                    )?;
                    Ok(OpenedFile(file))
                };
                let opened = match operation {
                    Some(context) => {
                        context
                            .run_blocking(budget, move |context| {
                                context.signal().check()?;
                                open()
                            })
                            .await
                    }
                    None => tokio::task::spawn_blocking(open).await.map_err(|error| {
                        HostError::new(
                            HostErrorCode::ProviderUnavailable,
                            "file open worker failed",
                        )
                        .with_detail("error", error.to_string())
                    })?,
                };
                match opened {
                    Ok(file) => {
                        let origin = crate::ByteStreamOrigin::File {
                            path: path.relative.clone(),
                            region: path.region.clone(),
                            mode,
                        };
                        let handle = self
                            .streams
                            .insert_stream(
                                crate::stream::store::ManagedStream::file(file.0, mode),
                                origin,
                            )
                            .await?;
                        if let Some(context) = operation {
                            self.streams.own_stream(&handle, context).await?;
                        }
                        Ok(FilesystemEntry::FileStream(crate::FileStreamRef::issued(
                            handle, path, mode,
                        )))
                    }
                    Err(error) => Err(error),
                }
            }
            FilesystemOperation::Sync { file } => self
                .streams
                .sync_file(&file, &request.budget, operation)
                .await
                .map(|()| FilesystemEntry::Unit),
            _ => unreachable!("managed filesystem operation"),
        };
        Ok(FilesystemResponse { id, result })
    }

    pub fn new(regions: WorkspaceRegionRegistry) -> Self {
        Self {
            regions: Arc::new(regions),
            streams: Default::default(),
        }
    }
}

impl FilesystemClient for LocalFilesystemClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<FilesystemResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: FilesystemRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            if matches!(
                request.operation,
                FilesystemOperation::Open { .. } | FilesystemOperation::Sync { .. }
            ) {
                return self.execute_managed(request, None).await;
            }
            let id = request.id;
            let result = execute_local_filesystem(&self.regions, request, None);
            Ok(FilesystemResponse { id, result })
        })
    }
}

fn execute_local_filesystem(
    regions: &WorkspaceRegionRegistry,
    request: FilesystemRequest,
    operation: Option<&crate::execution::OperationContext>,
) -> Result<FilesystemEntry, HostError> {
    require_filesystem_authority(&request)?;
    let broker = SandboxBroker::new(request.authority.sandbox);
    match request.operation {
        FilesystemOperation::Open { .. } | FilesystemOperation::Sync { .. } => {
            unreachable!("managed filesystem route")
        }
        FilesystemOperation::Canonicalize { path } => {
            let region = path.region.clone();
            let bound = regions.bind(&path)?;
            let relative = broker.canonicalize_file(&bound.root, &bound.relative)?;
            let relative = relative
                .to_str()
                .ok_or_else(|| {
                    HostError::new(HostErrorCode::IoFailure, "canonical path is not UTF-8")
                        .with_detail("io_kind", "InvalidData")
                })?
                .replace(std::path::MAIN_SEPARATOR, "/");
            Ok(FilesystemEntry::Path(crate::WorkspacePathRef::new(
                region,
                if relative.is_empty() { "." } else { &relative },
            )?))
        }
        FilesystemOperation::Read { path } => {
            let path = regions.bind(&path)?;
            broker
                .read_file(&path.root, &path.relative)
                .map(FilesystemEntry::Bytes)
        }
        FilesystemOperation::ReadLimited { path, max_bytes } => {
            let path = regions.bind(&path)?;
            let mut check = || {
                request.budget.check_time()?;
                if let Some(operation) = operation {
                    operation.signal().check()?;
                }
                Ok(())
            };
            broker
                .read_file_limited_controlled(&path.root, &path.relative, max_bytes, &mut check)
                .map(FilesystemEntry::LimitedRead)
        }
        FilesystemOperation::Write {
            path,
            contents,
            create_dirs,
        } => {
            let path = regions.bind(&path)?;
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
            let path = regions.bind(&path)?;
            broker.delete_file(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::CreateDir { path, recursive } => {
            let path = regions.bind_directory(&path)?;
            if recursive {
                broker.create_dir_all(&path.root, &path.relative)?;
            } else {
                broker.create_dir(&path.root, &path.relative)?;
            }
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::Rename {
            source,
            destination,
        } => {
            if source.region != destination.region {
                return Err(HostError::new(
                    HostErrorCode::AuthorityDenied,
                    "filesystem rename requires the same checked region",
                ));
            }
            let source = regions.bind(&source)?;
            let destination = regions.bind(&destination)?;
            broker.rename(&source.root, &source.relative, &destination.relative)?;
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::RemoveFile { path } => {
            let path = regions.bind(&path)?;
            broker.delete_file(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::RemoveDir { path } => {
            let path = regions.bind(&path)?;
            broker.remove_dir(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Unit)
        }
        FilesystemOperation::ReadDir { path } => {
            let region = path.region.clone();
            let parent = path.relative.clone();
            let path = regions.bind_directory(&path)?;
            let entries = broker
                .read_dir(&path.root, &path.relative)?
                .into_iter()
                .map(|entry| WorkspacePathRef::new(region.clone(), parent.join(entry)))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(FilesystemEntry::Entries(entries))
        }
        FilesystemOperation::ReadDirLimited {
            path,
            max_entries,
            max_bytes,
        } => {
            let region = path.region.clone();
            let path = regions.bind_directory(&path)?;
            let mut check = || {
                request.budget.check_time()?;
                if let Some(operation) = operation {
                    operation.signal().check()?;
                }
                Ok(())
            };
            let result = broker.read_dir_limited_controlled(
                &path.root,
                &path.relative,
                max_entries,
                max_bytes,
                &mut check,
            )?;
            Ok(FilesystemEntry::LimitedEntries(match result {
                crate::BoundedDirectoryList::Complete(names) => {
                    crate::BoundedDirectoryList::Complete(
                        names
                            .into_iter()
                            .map(|name| {
                                WorkspacePathRef::new(region.clone(), path.relative.join(name))
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    )
                }
                crate::BoundedDirectoryList::LimitExceeded {
                    max_entries,
                    max_bytes,
                } => crate::BoundedDirectoryList::LimitExceeded {
                    max_entries,
                    max_bytes,
                },
            }))
        }
        FilesystemOperation::Stat { path } => {
            let path = regions.bind_directory(&path)?;
            let metadata = broker.stat(&path.root, &path.relative)?;
            Ok(FilesystemEntry::Stat(FilesystemStat {
                is_file: metadata.is_file,
                is_dir: metadata.is_dir,
                len: if metadata.is_file { metadata.len } else { 0 },
            }))
        }
        FilesystemOperation::AtomicReplace { path, contents } => {
            let path = regions.bind(&path)?;
            broker.atomic_write(&path.root, &path.relative, &contents)?;
            Ok(FilesystemEntry::Unit)
        }
    }
}

fn require_filesystem_authority(request: &FilesystemRequest) -> Result<(), HostError> {
    if let FilesystemOperation::Open { path, mode } = &request.operation {
        for action in mode.rights() {
            require_region_grant(request, action, path)?;
        }
        return Ok(());
    }
    let (action, path) = match &request.operation {
        FilesystemOperation::Open { .. } => unreachable!(),
        FilesystemOperation::Sync { file } => {
            return require_region_grant(request, "sync", &file.path());
        }
        FilesystemOperation::Canonicalize { path } => ("canonicalize", path),
        FilesystemOperation::Read { path } | FilesystemOperation::ReadLimited { path, .. } => {
            ("read", path)
        }
        FilesystemOperation::Write { path, .. } => ("write", path),
        FilesystemOperation::Delete { path } => ("delete", path),
        FilesystemOperation::CreateDir { path, .. } => ("create_dir", path),
        FilesystemOperation::Rename { source, .. } => ("rename", source),
        FilesystemOperation::RemoveFile { path } => ("remove_file", path),
        FilesystemOperation::RemoveDir { path } => ("remove_dir", path),
        FilesystemOperation::ReadDir { path }
        | FilesystemOperation::ReadDirLimited { path, .. } => ("list", path),
        FilesystemOperation::Stat { path } => ("stat", path),
        FilesystemOperation::AtomicReplace { path, .. } => ("atomic_replace", path),
    };
    require_region_grant(request, action, path)
}

fn require_region_grant(
    request: &FilesystemRequest,
    action: &str,
    path: &WorkspacePathRef,
) -> Result<(), HostError> {
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

struct OpenedFile(std::fs::File);
impl crate::execution::OperationResponse for OpenedFile {
    fn external_outcome(&self) -> crate::execution::ExternalOutcome {
        crate::execution::ExternalOutcome::Confirmed
    }
}
