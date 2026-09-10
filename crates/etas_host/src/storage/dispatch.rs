use crate::{HostError, StorageOperationRef, WriteOutcome, execution::DispatchError};

impl<R, N> WriteOutcome<R, N> {
    pub(crate) fn from_dispatch_error(
        error: DispatchError,
        operation: StorageOperationRef,
        rejected: impl FnOnce(HostError) -> N,
    ) -> Self {
        match error {
            DispatchError::NotDispatched(error) => Self::NotCommitted {
                operation,
                reason: rejected(error),
            },
            DispatchError::OutcomeUnavailable(error) => Self::Unknown { operation, error },
        }
    }
}
