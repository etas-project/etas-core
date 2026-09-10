use crate::HostError;
use crate::execution::DispatchError;
use crate::session::{
    SessionMaintenanceClient, SessionMaintenanceOperation, SessionMaintenanceRequest,
    SessionMaintenanceResponse, SessionMaintenanceResult, SqliteSessionClient,
};
use std::{future::Future, pin::Pin};

impl SessionMaintenanceClient for SqliteSessionClient {
    type MaintainFuture<'a> =
        Pin<Box<dyn Future<Output = Result<SessionMaintenanceResponse, HostError>> + Send + 'a>>;
    fn maintain(&self, request: SessionMaintenanceRequest) -> Self::MaintainFuture<'_> {
        Box::pin(self.dispatch_maintenance(request, None))
    }
}
impl SqliteSessionClient {
    pub async fn maintain_scoped(
        &self,
        request: SessionMaintenanceRequest,
        context: &crate::execution::OperationContext,
    ) -> Result<SessionMaintenanceResponse, HostError> {
        self.dispatch_maintenance(request, Some(context)).await
    }
    async fn dispatch_maintenance(
        &self,
        request: SessionMaintenanceRequest,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<SessionMaintenanceResponse, HostError> {
        let bytes = request.validate(&self.worker.limits)?;
        let intent = match &request.operation {
            SessionMaintenanceOperation::Retain(intent) => Some(intent.operation.clone()),
            SessionMaintenanceOperation::Reconcile { .. } => None,
        };
        let limits = self.worker.limits.clone();
        let id = request.id;
        let result = self
            .worker
            .execute(
                context,
                id,
                request.trace,
                request.budget,
                bytes,
                move |connection, operation| {
                    let mut db = super::super::SessionDatabase {
                        connection,
                        operation: Some(operation),
                        limits: &limits,
                    };
                    Ok(SessionMaintenanceResponse {
                        id,
                        result: super::execute(&mut db, request.operation),
                    })
                },
            )
            .await;
        match (result, intent) {
            (Err(error), Some(operation)) => Ok(SessionMaintenanceResponse {
                id,
                result: Ok(SessionMaintenanceResult::Outcome(
                    crate::WriteOutcome::from_dispatch_error(
                        error,
                        operation,
                        crate::session::SessionRetentionRejection::Rejected,
                    ),
                )),
            }),
            (result, _) => result.map_err(DispatchError::into_host_error),
        }
    }
}
