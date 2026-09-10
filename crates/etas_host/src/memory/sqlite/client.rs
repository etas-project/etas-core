use super::{MemoryDatabase, schema};
use crate::execution::DispatchError;
use crate::storage::sqlite::{SqliteWorker, open_durable};
use crate::{HostError, MemoryClient, MemoryRequest, MemoryResponse, StorageLimits};
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

#[derive(Clone, Debug)]
pub struct SqliteMemoryClient {
    worker: SqliteWorker,
    path: PathBuf,
}
impl SqliteMemoryClient {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, HostError> {
        Self::open_with_limits(path, StorageLimits::default())
    }
    pub fn open_with_limits(
        path: impl AsRef<Path>,
        limits: StorageLimits,
    ) -> Result<Self, HostError> {
        let path = path.as_ref().to_path_buf();
        let connection = open_durable(&path, &limits, schema::initialize)?;
        Ok(Self {
            worker: SqliteWorker::new(connection, limits)?,
            path,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub async fn write_scoped(
        &self,
        request: crate::memory::MemoryWriteRequest,
        context: &crate::execution::OperationContext,
    ) -> Result<crate::memory::MemoryWriteResponse, HostError> {
        self.dispatch_write(request, Some(context)).await
    }
    async fn dispatch_write(
        &self,
        request: crate::memory::MemoryWriteRequest,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<crate::memory::MemoryWriteResponse, HostError> {
        let limits = self.worker.limits.clone();
        let bytes = request.storage_size(&limits)?;
        let pending = match &request.operation {
            crate::memory::MemoryWriteOperation::Mutate { key, mutation } => {
                Some(mutation.operation_ref(&request.store, key.clone(), &limits)?)
            }
            crate::memory::MemoryWriteOperation::Reconcile { .. } => None,
        };
        let id = request.id;
        let result = self
            .worker
            .execute(
                context,
                id,
                request.trace.clone(),
                request.budget.clone(),
                bytes,
                move |connection, operation| {
                    let mut db = MemoryDatabase {
                        connection,
                        operation: Some(operation),
                        limits: &limits,
                    };
                    Ok(crate::memory::MemoryWriteResponse {
                        id,
                        result: db.execute_write(&request.store, request.operation),
                    })
                },
            )
            .await;
        match (result, pending) {
            (Err(error), Some(operation)) => Ok(crate::memory::MemoryWriteResponse {
                id,
                result: Ok(crate::memory::MemoryWriteResult::Outcome(
                    crate::WriteOutcome::from_dispatch_error(
                        error,
                        operation,
                        crate::memory::MemoryNotCommitted::Rejected,
                    ),
                )),
            }),
            (result, _) => result.map_err(DispatchError::into_host_error),
        }
    }
    pub async fn execute_scoped(
        &self,
        request: MemoryRequest,
        context: &crate::execution::OperationContext,
    ) -> Result<MemoryResponse, HostError> {
        self.dispatch(request, Some(context)).await
    }
    async fn dispatch(
        &self,
        request: MemoryRequest,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<MemoryResponse, HostError> {
        let bytes = request.storage_size(&self.worker.limits)?;
        let id = request.id;
        let limits = self.worker.limits.clone();
        self.worker
            .execute(
                context,
                id,
                request.trace.clone(),
                request.budget.clone(),
                bytes,
                move |connection, operation| {
                    let mut db = MemoryDatabase {
                        connection,
                        operation: Some(operation),
                        limits: &limits,
                    };
                    let result = db.execute_operation(request);
                    Ok(MemoryResponse { id, result })
                },
            )
            .await
            .map_err(DispatchError::into_host_error)
    }
}
impl MemoryClient for SqliteMemoryClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<MemoryResponse, HostError>> + Send + 'a>>;
    fn execute(&self, request: MemoryRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.dispatch(request, None))
    }
    type WriteFuture<'a> = Pin<
        Box<dyn Future<Output = Result<crate::memory::MemoryWriteResponse, HostError>> + Send + 'a>,
    >;
    fn write(&self, request: crate::memory::MemoryWriteRequest) -> Self::WriteFuture<'_> {
        Box::pin(self.dispatch_write(request, None))
    }
}
