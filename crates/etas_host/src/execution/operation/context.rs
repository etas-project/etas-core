use super::super::{CancelSignal, ScopeId};
use super::OperationId;
use crate::{HostRequestId, TraceContext};

/// Local-only correlation and cancellation; never part of a provider payload.
#[derive(Clone, Debug)]
pub struct OperationContext {
    scope: ScopeId,
    operation: OperationId,
    parent: Option<OperationId>,
    request: Option<HostRequestId>,
    trace: TraceContext,
    signal: CancelSignal,
}

impl OperationContext {
    pub(crate) fn register_work(&self) -> Result<super::OperationRegistration, crate::HostError> {
        self.signal
            .register_child(self.operation, self.request, self.trace.clone())
    }
    /// Transfer backend work to an owned task. Dropping the receiver never drops
    /// the registration: it remains in the scope until the task actually settles.
    pub async fn supervise<T, F, Fut>(&self, job: F) -> Result<T, crate::HostError>
    where
        T: super::OperationResponse + Send + 'static,
        F: FnOnce(OperationContext) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<T, crate::HostError>> + Send + 'static,
    {
        let registration =
            self.signal
                .register_child(self.operation, self.request, self.trace.clone())?;
        let context = registration.context().clone();
        let task = tokio::spawn(async move {
            if let Err(error) = registration.begin_dispatch() {
                registration.complete(super::ExternalOutcome::NotDispatched, vec![])?;
                return Err(error);
            }
            let result = job(context).await;
            // The job owns cleanup and must not return until its local work stops.
            let outcome = match &result {
                Ok(response) => response.external_outcome(),
                Err(_) => super::ExternalOutcome::Unknown,
            };
            registration.complete(outcome, vec![])?;
            result
        });
        task.await.map_err(|error| {
            crate::HostError::new(
                crate::HostErrorCode::ProviderUnavailable,
                "owned host operation task failed",
            )
            .with_detail("error", error.to_string())
        })?
    }
    pub fn record_progress(&self, completed_units: u64) -> Result<(), crate::HostError> {
        self.signal.record_progress(self.operation, completed_units)
    }
    /// Record cleanup failure without publishing termination. The resource owner
    /// must stay alive and complete only once cleanup is actually confirmed.
    pub(crate) fn record_cleanup_error(
        &self,
        error: crate::HostError,
    ) -> Result<(), crate::HostError> {
        self.signal.record_cleanup_error(self.operation, error)
    }
    pub(crate) fn new(
        scope: ScopeId,
        operation: OperationId,
        parent: Option<OperationId>,
        request: Option<HostRequestId>,
        trace: TraceContext,
        signal: CancelSignal,
    ) -> Self {
        Self {
            scope,
            operation,
            parent,
            request,
            trace,
            signal,
        }
    }
    pub fn scope(&self) -> ScopeId {
        self.scope
    }
    pub fn operation(&self) -> OperationId {
        self.operation
    }
    pub fn parent(&self) -> Option<OperationId> {
        self.parent
    }
    pub fn request(&self) -> Option<HostRequestId> {
        self.request
    }
    pub fn trace(&self) -> &TraceContext {
        &self.trace
    }
    pub fn signal(&self) -> &CancelSignal {
        &self.signal
    }
}
