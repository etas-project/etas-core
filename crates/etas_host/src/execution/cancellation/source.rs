use tokio_util::sync::CancellationToken;

use super::super::ExecutionScope;
use super::{CancellationCause, CancellationReason};
use crate::HostError;

#[derive(Clone, Debug)]
pub struct CancelSource {
    scope: ExecutionScope,
}

impl CancelSource {
    pub(crate) fn new(scope: ExecutionScope) -> Self {
        Self { scope }
    }

    /// Idempotent and nonblocking. It does not claim that work has terminated.
    pub fn stop(&self, reason: CancellationReason) -> Result<(), HostError> {
        self.scope.request_stop(reason)
    }
}

/// Observation only: adapters cannot use this handle to stop a parent or sibling.
#[derive(Clone, Debug)]
pub struct CancelSignal {
    scope: ExecutionScope,
    token: CancellationToken,
}

impl CancelSignal {
    pub(crate) fn parent_scope(&self) -> Result<Option<ExecutionScope>, HostError> {
        self.scope.parent()
    }

    pub(crate) async fn release_requested(
        &self,
        operation: super::super::OperationId,
    ) -> Result<(), HostError> {
        let mut changed = self.scope.subscribe();
        loop {
            if self.scope.resource_release_ready(operation)? {
                return Ok(());
            }
            changed
                .changed()
                .await
                .map_err(|_| super::super::scope::invalid_state("scope release channel closed"))?;
        }
    }

    pub(crate) fn mark_resource(
        &self,
        operation: super::super::OperationId,
    ) -> Result<(), HostError> {
        self.scope.mark_resource(operation)
    }
    pub(crate) fn register_child(
        &self,
        parent: super::super::OperationId,
        request: Option<crate::HostRequestId>,
        trace: crate::TraceContext,
    ) -> Result<super::super::OperationRegistration, HostError> {
        self.scope.register(Some(parent), request, trace)
    }
    pub(crate) fn record_progress(
        &self,
        operation: super::super::OperationId,
        completed_units: u64,
    ) -> Result<(), HostError> {
        self.scope.record_progress(operation, completed_units)
    }
    pub(crate) fn record_cleanup_error(
        &self,
        operation: super::super::OperationId,
        error: HostError,
    ) -> Result<(), HostError> {
        self.scope.record_cleanup_error(operation, error)
    }
    pub(crate) fn new(scope: ExecutionScope, token: CancellationToken) -> Self {
        Self { scope, token }
    }

    pub fn cause(&self) -> Result<Option<CancellationCause>, HostError> {
        self.scope.cause()
    }

    pub fn is_cancelled(&self) -> bool {
        self.scope.stop_requested_in_tree() && self.token.is_cancelled()
    }

    pub fn check(&self) -> Result<(), HostError> {
        if self.cause()?.is_some() {
            Err(HostError::new(
                crate::HostErrorCode::Cancelled,
                "execution scope was cancelled",
            ))
        } else {
            Ok(())
        }
    }

    pub async fn cancelled(&self) -> Result<CancellationCause, HostError> {
        self.token.cancelled().await;
        self.cause()?.ok_or_else(|| {
            super::super::scope::invalid_state("cancellation notification has no published cause")
        })
    }
}
