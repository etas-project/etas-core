//! Invocation-local ownership and cancellation. No live state is serializable.
mod cancellation;
mod operation;
mod scope;
mod shutdown;

pub use cancellation::{CancelSignal, CancelSource, CancellationCause, CancellationReason};
pub(crate) use operation::DispatchError;
pub(crate) use operation::run_blocking_managed;
pub use operation::{
    ExternalOutcome, OperationContext, OperationId, OperationRegistration, OperationReport,
    OperationResponse,
};
pub use scope::{ExecutionScope, ScopeId, ScopeState};
pub use shutdown::{PendingWork, ScopeOutcome, StopWait, TerminationReport};

#[cfg(test)]
mod tests;
