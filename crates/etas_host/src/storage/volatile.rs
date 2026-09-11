use super::limits::{Admission, Reservation, StorageLimits};
use crate::execution::{DispatchError, ExternalOutcome, OperationContext, OperationResponse};
use crate::{ExecutionBudget, HostError, HostErrorCode, HostRequestId, TraceContext};
use std::sync::Arc;

/// Per-backend storage admission layered on the shared managed blocking executor.
#[derive(Clone, Debug)]
pub(crate) struct VolatileExecutor {
    admission: Arc<Admission>,
    limits: StorageLimits,
}
impl Default for VolatileExecutor {
    fn default() -> Self {
        Self::new(StorageLimits::default())
    }
}
impl VolatileExecutor {
    pub fn new(limits: StorageLimits) -> Self {
        Self {
            admission: Arc::new(Admission::new(limits.clone())),
            limits,
        }
    }
    pub async fn execute<T, F>(
        &self,
        context: Option<&OperationContext>,
        id: HostRequestId,
        trace: TraceContext,
        budget: ExecutionBudget,
        request_bytes: usize,
        job: F,
    ) -> Result<T, DispatchError>
    where
        T: OperationResponse + Send + 'static,
        F: FnOnce(OperationContext) -> Result<T, HostError> + Send + 'static,
    {
        use DispatchError::NotDispatched;
        budget.check_time().map_err(NotDispatched)?;
        if let Some(context) = context {
            context.signal().check().map_err(NotDispatched)?;
        }
        // The request, one bounded encoding buffer, and a pending response are
        // retained until delivery. No unbounded admission wait list exists.
        let bytes = request_bytes
            .checked_add(self.limits.max_value_bytes)
            .and_then(|bytes| bytes.checked_add(self.limits.max_result_bytes))
            .ok_or_else(|| {
                NotDispatched(HostError::new(
                    HostErrorCode::BudgetExceeded,
                    "storage reservation overflow",
                ))
            })?;
        let reservation = self.admission.try_reserve(bytes).map_err(NotDispatched)?;
        let response = crate::execution::run_blocking_managed(
            context,
            Some(id),
            trace,
            budget,
            move |context| {
                Ok(ReservedResponse {
                    value: job(context)?,
                    _reservation: reservation,
                })
            },
        )
        .await?;
        Ok(response.value)
    }
}
struct ReservedResponse<T> {
    value: T,
    _reservation: Reservation,
}
impl<T: OperationResponse> OperationResponse for ReservedResponse<T> {
    fn external_outcome(&self) -> ExternalOutcome {
        self.value.external_outcome()
    }
}
