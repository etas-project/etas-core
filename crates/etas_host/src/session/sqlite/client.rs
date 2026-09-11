use super::{SessionDatabase, initialize_schema};
use crate::execution::DispatchError;
use crate::storage::sqlite::{SqliteWorker, open_durable};
use crate::{HostError, SessionClient, SessionRequest, SessionResponse, StorageLimits};
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

#[derive(Clone, Debug)]
pub struct SqliteSessionClient {
    pub(super) worker: SqliteWorker,
    path: PathBuf,
}
enum PendingMutation {
    Context(crate::StorageOperationRef),
    Write(crate::StorageOperationRef),
}
impl SqliteSessionClient {
    pub async fn write_scoped(
        &self,
        request: crate::session::SessionWriteRequest,
        context: &crate::execution::OperationContext,
    ) -> Result<crate::session::SessionWriteResponse, HostError> {
        self.dispatch_write(request, Some(context)).await
    }
    async fn dispatch_write(
        &self,
        request: crate::session::SessionWriteRequest,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<crate::session::SessionWriteResponse, HostError> {
        use crate::session::{SessionWriteOperation, SessionWriteResponse};
        let bytes = request.storage_size(&self.worker.limits)?;
        let id = request.id;
        let limits = self.worker.limits.clone();
        let pending = match &request.operation {
            SessionWriteOperation::PublishContext(p) => {
                Some(PendingMutation::Context(p.operation.clone()))
            }
            SessionWriteOperation::Resolve { config, key } => Some(PendingMutation::Write(
                crate::session::resolve_operation_ref(config, key.clone(), &limits)?,
            )),
            SessionWriteOperation::Append { message, key } => Some(PendingMutation::Write(
                crate::session::append_operation_ref(message, key.clone(), &limits)?,
            )),
            _ => None,
        };
        let result = self
            .worker
            .execute(
                context,
                id,
                request.trace,
                request.budget,
                bytes,
                move |connection, operation| {
                    let mut db = SessionDatabase {
                        connection,
                        operation: Some(operation),
                        limits: &limits,
                    };
                    Ok(SessionWriteResponse {
                        id,
                        result: db.execute_write(request.operation),
                    })
                },
            )
            .await;
        match (result, pending) {
            (Err(error), Some(PendingMutation::Context(operation))) => Ok(SessionWriteResponse {
                id,
                result: Ok(crate::session::SessionWriteResult::Context(
                    crate::WriteOutcome::from_dispatch_error(
                        error,
                        operation,
                        crate::session::SessionContextRejection::Rejected,
                    ),
                )),
            }),
            (Err(error), Some(PendingMutation::Write(operation))) => Ok(SessionWriteResponse {
                id,
                result: Ok(crate::session::SessionWriteResult::Outcome(
                    crate::WriteOutcome::from_dispatch_error(error, operation, |error| error),
                )),
            }),
            (result, _) => result.map_err(DispatchError::into_host_error),
        }
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self, HostError> {
        Self::open_with_limits(path, StorageLimits::default())
    }
    pub fn open_with_limits(
        path: impl AsRef<Path>,
        limits: StorageLimits,
    ) -> Result<Self, HostError> {
        let path = path.as_ref().to_path_buf();
        let connection = open_durable(&path, &limits, initialize_schema)?;
        Ok(Self {
            worker: SqliteWorker::new(connection, limits)?,
            path,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub async fn execute_scoped(
        &self,
        request: SessionRequest,
        context: &crate::execution::OperationContext,
    ) -> Result<SessionResponse, HostError> {
        self.dispatch(request, Some(context)).await
    }
    async fn dispatch(
        &self,
        request: SessionRequest,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<SessionResponse, HostError> {
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
                    let mut db = SessionDatabase {
                        connection,
                        operation: Some(operation),
                        limits: &limits,
                    };
                    let result = db.execute_operation(request.operation);
                    Ok(SessionResponse { id, result })
                },
            )
            .await
            .map_err(DispatchError::into_host_error)
    }
}
impl SessionClient for SqliteSessionClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<SessionResponse, HostError>> + Send + 'a>>;
    fn execute(&self, request: SessionRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.dispatch(request, None))
    }
    type WriteFuture<'a> = Pin<
        Box<
            dyn Future<Output = Result<crate::session::SessionWriteResponse, HostError>>
                + Send
                + 'a,
        >,
    >;
    fn write(&self, request: crate::session::SessionWriteRequest) -> Self::WriteFuture<'_> {
        Box::pin(self.dispatch_write(request, None))
    }
}
