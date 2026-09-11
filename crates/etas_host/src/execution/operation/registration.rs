use super::super::{CancellationReason, ExecutionScope};
use super::{ExternalOutcome, OperationContext};
use crate::HostError;

/// Move this owner into the actual supervisor. Dropping it never implies cleanup.
#[derive(Debug)]
pub struct OperationRegistration {
    scope: ExecutionScope,
    context: OperationContext,
    completed: bool,
}

impl OperationRegistration {
    pub(crate) fn new(scope: ExecutionScope, context: OperationContext) -> Self {
        Self {
            scope,
            context,
            completed: false,
        }
    }
    pub fn context(&self) -> &OperationContext {
        &self.context
    }

    /// Serialized with stop. Adapters must also observe the signal at the boundary.
    pub fn begin_dispatch(&self) -> Result<(), HostError> {
        self.scope.begin_dispatch(self.context.operation())
    }

    /// Publish exactly once, after owned local work and cleanup have settled.
    pub fn complete(
        mut self,
        outcome: ExternalOutcome,
        cleanup_errors: Vec<HostError>,
    ) -> Result<(), HostError> {
        self.scope
            .complete_operation(self.context.operation(), outcome, cleanup_errors)?;
        self.completed = true;
        Ok(())
    }
}

impl Drop for OperationRegistration {
    fn drop(&mut self) {
        if !self.completed {
            // Retain pending accounting when the work's owner disappears.
            let _ = self.scope.request_stop(CancellationReason::OwnerDropped);
            let _ = self.scope.abandon_operation(self.context.operation());
        }
    }
}
