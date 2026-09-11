use super::DispatchError;
use super::DispatchError::{NotDispatched, OutcomeUnavailable};
use super::SqliteOperation;
use crate::execution::{ExecutionScope, ExternalOutcome, OperationContext, OperationResponse};
use crate::storage::limits::{Admission, StorageLimits};
use crate::{ExecutionBudget, HostError, HostErrorCode, HostRequestId, TraceContext};
use rusqlite::Connection;
use std::sync::{
    Arc,
    mpsc::{self, SyncSender},
};

type Job = Box<dyn FnOnce(Result<&mut Connection, HostError>) -> bool + Send>;
#[derive(Clone)]
pub(crate) struct SqliteWorker {
    sender: SyncSender<Job>,
    admission: Arc<Admission>,
    pub limits: StorageLimits,
}
impl std::fmt::Debug for SqliteWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteWorker")
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}
impl SqliteWorker {
    pub fn new(mut connection: Connection, limits: StorageLimits) -> Result<Self, HostError> {
        limits.validate()?;
        super::config::configure_worker(&connection, &limits)?;
        let (sender, receiver) = mpsc::sync_channel::<Job>(limits.max_pending_jobs);
        std::thread::Builder::new()
            .name("etas-sqlite".into())
            .spawn(move || {
                while let Ok(job) = receiver.recv() {
                    // A panicking job quarantines this connection; queued deliveries
                    // close rather than using a possibly half-open transaction.
                    if !job(Ok(&mut connection)) {
                        break;
                    }
                }
            })
            .map_err(|_| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "cannot start SQLite connection owner",
                )
            })?;
        Ok(Self {
            sender,
            admission: Arc::new(Admission::new(limits.clone())),
            limits,
        })
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
        F: FnOnce(&mut Connection, &SqliteOperation) -> Result<T, HostError> + Send + 'static,
    {
        budget.check_time().map_err(NotDispatched)?;
        if let Some(context) = context {
            context.signal().check().map_err(NotDispatched)?;
        }
        let bytes = request_bytes
            .checked_add(super::config::scratch_bytes(&self.limits).map_err(NotDispatched)?)
            .and_then(|bytes| bytes.checked_add(self.limits.max_result_bytes))
            .ok_or_else(|| {
                HostError::new(
                    HostErrorCode::BudgetExceeded,
                    "storage reservation overflow",
                )
            })
            .map_err(NotDispatched)?;
        let permit = self.admission.try_reserve(bytes).map_err(NotDispatched)?;
        let registration = match context {
            Some(context) => context.register_work().map_err(NotDispatched)?,
            None => {
                let scope = ExecutionScope::new();
                let registration = scope
                    .register(None, Some(id), trace)
                    .map_err(NotDispatched)?;
                scope.finish_body(true).map_err(NotDispatched)?;
                registration
            }
        };
        let control = SqliteOperation {
            context: registration.context().clone(),
            budget,
        };
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let job = Box::new(move |connection: Result<&mut Connection, HostError>| {
            let connection = match connection {
                Ok(connection) => connection,
                Err(error) => {
                    let complete = registration.complete(ExternalOutcome::NotDispatched, vec![]);
                    let _ = sender.send((complete.and(Err(error)).map_err(NotDispatched), permit));
                    return true;
                }
            };
            if let Err(error) = control.check().and_then(|()| registration.begin_dispatch()) {
                let complete = registration.complete(ExternalOutcome::NotDispatched, vec![]);
                let _ = sender.send((complete.and(Err(error)).map_err(NotDispatched), permit));
                return true;
            }
            let progress = control.clone();
            connection.progress_handler(1000, Some(move || progress.check().is_err()));
            let result = job(connection, &control);
            connection.progress_handler(0, None::<fn() -> bool>);
            // Business cancellation is disabled while settling a transaction.
            // Never hand a connection with failed cleanup to the next job.
            let mut cleanup_errors = Vec::new();
            if !connection.is_autocommit() {
                if let Err(error) = connection.execute_batch("ROLLBACK") {
                    cleanup_errors.push(
                        HostError::new(
                            HostErrorCode::ProviderUnavailable,
                            "SQLite rollback failed",
                        )
                        .with_detail("error", error.to_string()),
                    );
                } else if !connection.is_autocommit() {
                    cleanup_errors.push(HostError::new(
                        HostErrorCode::ProviderUnavailable,
                        "SQLite transaction remained active after rollback",
                    ));
                }
            }
            let reusable = cleanup_errors.is_empty();
            let outcome = match &result {
                Ok(response) => response.external_outcome(),
                Err(_) => ExternalOutcome::Unknown,
            };
            let settled = registration.complete(outcome, cleanup_errors);
            // Capacity includes pending response storage until delivery/drop.
            let _ = sender.send((settled.and(result).map_err(OutcomeUnavailable), permit));
            reusable
        }) as Job;
        if let Err(rejected) = self.sender.try_send(job) {
            let error = HostError::new(
                HostErrorCode::ProviderUnavailable,
                "SQLite worker unavailable",
            );
            // Rejected work never acquired a connection; settle its registration
            // explicitly rather than dropping it as an abandoned running job.
            let (mpsc::TrySendError::Full(job) | mpsc::TrySendError::Disconnected(job)) = rejected;
            job(Err(error));
        }
        let (result, _permit) = receiver.await.map_err(|_| {
            OutcomeUnavailable(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "SQLite worker failed before delivery",
            ))
        })?;
        result
    }
}
