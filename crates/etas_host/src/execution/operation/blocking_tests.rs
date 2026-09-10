use super::*;
use crate::console::{ConsoleResponse, ConsoleResult};
use crate::execution::{CancellationReason, ExecutionScope};
use crate::{HostRequestId, TraceContext, TraceId};
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[test]
fn cancellation_while_blocking_job_is_queued_proves_non_dispatch() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        // Occupy the real blocking pool so the managed job is queued, not running.
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let busy = tokio::task::spawn_blocking(move || {
            started.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(5)).unwrap();
        });
        ready.await.unwrap();
        let scope = ExecutionScope::new();
        let parent = scope
            .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
            .unwrap();
        parent.begin_dispatch().unwrap();
        let executed = Arc::new(AtomicBool::new(false));
        let mark = executed.clone();
        let context = parent.context().clone();
        let job = context.run_blocking_dispatched(Default::default(), move |_| {
            mark.store(true, Ordering::SeqCst);
            Ok(ConsoleResponse {
                id: HostRequestId(1),
                result: ConsoleResult::Written,
            })
        });
        tokio::pin!(job);
        std::future::poll_fn(|cx| {
            assert!(job.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        scope
            .cancel_source()
            .stop(CancellationReason::Requested)
            .unwrap();
        release.send(()).unwrap();
        busy.await.unwrap();
        let error = tokio::time::timeout(Duration::from_secs(2), job)
            .await
            .unwrap()
            .unwrap_err();
        assert!(matches!(
            error,
            DispatchError::NotDispatched(HostError {
                code: HostErrorCode::Cancelled,
                ..
            })
        ));
        assert!(!executed.load(Ordering::SeqCst));
        // The parent was dispatched, but the queued child was not.
        parent.complete(ExternalOutcome::Confirmed, vec![]).unwrap();
        scope.finish_body(true).unwrap();
        let report = scope.join().await.unwrap();
        assert_eq!(report.operations().len(), 2);
        assert_eq!(
            report.operations()[1].outcome(),
            Some(&ExternalOutcome::NotDispatched)
        );
    });
}

#[tokio::test(flavor = "current_thread")]
async fn cancellation_error_after_running_job_does_not_prove_non_dispatch() {
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    parent.begin_dispatch().unwrap();
    let executed = Arc::new(AtomicBool::new(false));
    let mark = executed.clone();
    let error = parent
        .context()
        .run_blocking_dispatched::<ConsoleResponse, _>(Default::default(), move |_| {
            mark.store(true, Ordering::SeqCst);
            Err(HostError::new(
                HostErrorCode::Cancelled,
                "cancelled after domain execution",
            ))
        })
        .await
        .unwrap_err();
    assert!(executed.load(Ordering::SeqCst));
    assert!(matches!(
        error,
        DispatchError::OutcomeUnavailable(HostError {
            code: HostErrorCode::Cancelled,
            ..
        })
    ));
    parent.complete(ExternalOutcome::Unknown, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    let report = scope.join().await.unwrap();
    assert_eq!(
        report.operations()[1].outcome(),
        Some(&ExternalOutcome::Unknown)
    );
}
