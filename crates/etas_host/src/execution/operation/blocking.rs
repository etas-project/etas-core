use std::sync::{Arc, LazyLock};

use super::{DispatchError, ExternalOutcome, OperationContext, OperationResponse};
use crate::{HostError, HostErrorCode, HostRequestId, TraceContext};
use DispatchError::{NotDispatched, OutcomeUnavailable};

static BLOCKING_CAPACITY: LazyLock<Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(32)));

#[cfg(test)]
#[path = "blocking_tests.rs"]
mod tests;

impl OperationContext {
    /// Bounded admission. The actual blocking worker owns the registration,
    /// including when its async receiver or the embedding runtime is dropped.
    pub async fn run_blocking<T, F>(
        &self,
        budget: crate::ExecutionBudget,
        job: F,
    ) -> Result<T, HostError>
    where
        T: OperationResponse + Send + 'static,
        F: FnOnce(OperationContext) -> Result<T, HostError> + Send + 'static,
    {
        self.run_blocking_dispatched(budget, job)
            .await
            .map_err(DispatchError::into_host_error)
    }

    pub(crate) async fn run_blocking_dispatched<T, F>(
        &self,
        budget: crate::ExecutionBudget,
        job: F,
    ) -> Result<T, DispatchError>
    where
        T: OperationResponse + Send + 'static,
        F: FnOnce(OperationContext) -> Result<T, HostError> + Send + 'static,
    {
        run_blocking_managed(
            Some(self),
            self.request(),
            self.trace().clone(),
            budget,
            job,
        )
        .await
    }
}

pub(crate) async fn run_blocking_managed<T, F>(
    context: Option<&OperationContext>,
    request: Option<HostRequestId>,
    trace: TraceContext,
    budget: crate::ExecutionBudget,
    job: F,
) -> Result<T, DispatchError>
where
    T: OperationResponse + Send + 'static,
    F: FnOnce(OperationContext) -> Result<T, HostError> + Send + 'static,
{
    if let Some(context) = context {
        context.signal().check().map_err(NotDispatched)?;
    }
    budget.check_time().map_err(NotDispatched)?;
    let runtime = tokio::runtime::Handle::try_current().map_err(|_| {
        NotDispatched(HostError::new(
            HostErrorCode::ProviderUnavailable,
            "managed blocking execution requires a Tokio runtime",
        ))
    })?;
    // Waiting futures retain their payload too; do not create an unbounded
    // semaphore wait list outside the admitted worker capacity.
    let permit = BLOCKING_CAPACITY
        .clone()
        .try_acquire_owned()
        .map_err(|error| {
            NotDispatched(
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "blocking operation capacity unavailable",
                )
                .with_detail("admission", error.to_string()),
            )
        })?;
    let registration = match context {
        Some(context) => context.register_work().map_err(NotDispatched)?,
        None => {
            let scope = crate::execution::ExecutionScope::new();
            let registration = scope
                .register(None, request, trace)
                .map_err(NotDispatched)?;
            scope.finish_body(true).map_err(NotDispatched)?;
            registration
        }
    };
    let context = registration.context().clone();
    runtime
        .spawn_blocking(move || {
            let _permit = permit;
            if let Err(error) = budget
                .check_time()
                .and_then(|()| context.signal().check())
                .and_then(|()| registration.begin_dispatch())
            {
                registration
                    .complete(ExternalOutcome::NotDispatched, vec![])
                    .map_err(OutcomeUnavailable)?;
                return Err(NotDispatched(error));
            }
            let result = job(context);
            let outcome = match &result {
                Ok(response) => response.external_outcome(),
                Err(_) => ExternalOutcome::Unknown,
            };
            registration
                .complete(outcome, vec![])
                .map_err(OutcomeUnavailable)?;
            result.map_err(OutcomeUnavailable)
        })
        .await
        .map_err(|error| {
            OutcomeUnavailable(
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "blocking operation worker failed",
                )
                .with_detail("error", error.to_string()),
            )
        })?
}
