use super::process_tree::ProcessTreeController;
use crate::{HostError, HostErrorCode, execution::OperationContext};
use tokio::process::Child;

trait ProcessCleanup {
    fn kill(&mut self) -> Result<(), HostError>;
    fn try_reap(&mut self) -> Result<bool, HostError>;
    fn reap(&mut self) -> impl std::future::Future<Output = Result<(), HostError>> + Send;
}

struct ChildCleanup<'a> {
    child: &'a mut Child,
    tree: Option<ProcessTreeController>,
}
impl ProcessCleanup for ChildCleanup<'_> {
    fn kill(&mut self) -> Result<(), HostError> {
        let tree = match self.tree {
            Some(tree) => tree,
            None => ProcessTreeController::for_child(self.child, "command cleanup")?,
        };
        self.tree = Some(tree);
        tree.kill(self.child)
    }
    fn try_reap(&mut self) -> Result<bool, HostError> {
        self.child
            .try_wait()
            .map(|status| status.is_some())
            .map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to inspect command termination",
                )
                .with_detail("error", error.to_string())
            })
    }
    async fn reap(&mut self) -> Result<(), HostError> {
        self.child.wait().await.map(|_| ()).map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to reap terminated command",
            )
            .with_detail("error", error.to_string())
        })
    }
}

pub(super) async fn terminate_and_reap(
    child: &mut Child,
    tree: ProcessTreeController,
    operation: Option<&OperationContext>,
) -> Result<(), HostError> {
    settle(
        &mut ChildCleanup {
            child,
            tree: Some(tree),
        },
        operation,
    )
    .await
}

pub(super) async fn terminate_uninitialized_child(
    child: &mut Child,
    operation: Option<&OperationContext>,
) -> Result<(), HostError> {
    settle(&mut ChildCleanup { child, tree: None }, operation).await
}

async fn settle(
    process: &mut impl ProcessCleanup,
    operation: Option<&OperationContext>,
) -> Result<(), HostError> {
    let mut first_error = None;
    let mut reaped = false;
    let mut reap_error_recorded = false;
    // Retain the child and ignore further business cancellation until cleanup
    // is confirmed. The owning execution scope remains pending during recovery.
    let mut kill_error_recorded = false;
    while let Err(error) = process.kill() {
        record_first_failure(operation, error, &mut first_error, &mut kill_error_recorded);
        if !reaped {
            match process.try_reap() {
                Ok(done) => reaped = done,
                Err(error) => record_first_failure(
                    operation,
                    error,
                    &mut first_error,
                    &mut reap_error_recorded,
                ),
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    while !reaped {
        let error = match process.reap().await {
            Ok(()) => {
                reaped = true;
                continue;
            }
            Err(error) => error,
        };
        record_first_failure(operation, error, &mut first_error, &mut reap_error_recorded);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn record_first_failure(
    operation: Option<&OperationContext>,
    error: HostError,
    first_error: &mut Option<HostError>,
    recorded: &mut bool,
) {
    // One failure per phase bounds the report even for a permanently failing
    // OS operation. Completion must preserve this historical evidence.
    if *recorded {
        return;
    }
    *recorded = true;
    let reporting = operation
        .map(|operation| operation.record_cleanup_error(error.clone()))
        .transpose();
    first_error.get_or_insert(match reporting {
        Ok(_) => error,
        Err(reporting) => reporting,
    });
}

#[cfg(test)]
mod tests;
