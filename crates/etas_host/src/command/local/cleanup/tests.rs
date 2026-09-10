use super::*;
use crate::{
    HostRequestId, TraceContext, TraceId,
    execution::{ExecutionScope, ExternalOutcome, StopWait},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Notify;

struct FaultingProcess {
    kill_failed: bool,
    reap_failed: bool,
    waiting: Arc<Notify>,
    release: Arc<Notify>,
}
impl ProcessCleanup for FaultingProcess {
    fn try_reap(&mut self) -> Result<bool, HostError> {
        Ok(false)
    }
    fn kill(&mut self) -> Result<(), HostError> {
        if !self.kill_failed {
            self.kill_failed = true;
            return Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "injected kill failure",
            ));
        }
        Ok(())
    }
    async fn reap(&mut self) -> Result<(), HostError> {
        if !self.reap_failed {
            self.reap_failed = true;
            return Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "injected reap failure",
            ));
        }
        self.waiting.notify_one();
        self.release.notified().await;
        Ok(())
    }
}

#[tokio::test(flavor = "current_thread")]
async fn cleanup_failure_remains_pending_and_evidence_survives_completion() {
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    parent.begin_dispatch().unwrap();
    let context = parent.context().clone();
    let waiting = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let mut process = FaultingProcess {
        kill_failed: false,
        reap_failed: false,
        waiting: waiting.clone(),
        release: release.clone(),
    };
    let task = tokio::spawn(async move {
        context
            .supervise(move |context| async move {
                settle(&mut process, Some(&context)).await?;
                Ok(crate::console::ConsoleResponse {
                    id: HostRequestId(1),
                    result: crate::console::ConsoleResult::Written,
                })
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), waiting.notified())
        .await
        .unwrap();
    parent.complete(ExternalOutcome::Unknown, vec![]).unwrap();
    scope.finish_body(false).unwrap();
    let StopWait::TimedOut(pending) = scope
        .wait_stopped(tokio::time::Instant::now())
        .await
        .unwrap()
    else {
        panic!("unreaped child cannot terminate the scope")
    };
    let child = pending
        .operations()
        .iter()
        .find(|report| report.parent().is_some())
        .unwrap();
    assert!(child.outcome().is_none());
    assert!(!child.owner_lost());
    assert_eq!(child.cleanup_errors().len(), 2);
    release.notify_one();
    assert_eq!(
        task.await.unwrap().unwrap_err().message,
        "injected kill failure"
    );
    let report = tokio::time::timeout(Duration::from_secs(2), scope.join())
        .await
        .unwrap()
        .unwrap();
    let child = report
        .operations()
        .iter()
        .find(|report| report.parent().is_some())
        .unwrap();
    assert_eq!(
        child.cleanup_errors().len(),
        2,
        "supervise must not overwrite cleanup errors with an empty vector"
    );
    assert!(child.outcome().is_some());
}
