use std::{
    sync::{Arc, Barrier},
    time::Duration,
};

use super::*;
use crate::{HostRequestId, TraceContext, TraceId};

fn register(scope: &ExecutionScope) -> OperationRegistration {
    scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_blocking_waiter_retains_real_worker_until_completion() {
    let scope = ExecutionScope::new();
    let parent = register(&scope);
    parent.begin_dispatch().unwrap();
    let context = parent.context().clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let waiter = tokio::spawn(async move {
        context
            .run_blocking(crate::ExecutionBudget::default(), move |_| {
                started.send(()).unwrap();
                wait.recv().unwrap();
                Ok(crate::console::ConsoleResponse {
                    id: HostRequestId(1),
                    result: crate::console::ConsoleResult::Written,
                })
            })
            .await
    });
    ready.await.unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    drop(parent);
    scope.finish_body(true).unwrap();
    assert!(matches!(
        scope
            .wait_stopped(tokio::time::Instant::now())
            .await
            .unwrap(),
        StopWait::TimedOut(_)
    ));
    release.send(()).unwrap();
    let report = tokio::time::timeout(Duration::from_secs(2), scope.join())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(report.operations().len(), 2);
    assert_eq!(
        report.operations()[0].outcome(),
        Some(&ExternalOutcome::Unknown)
    );
    assert_eq!(
        report.operations()[1].outcome(),
        Some(&ExternalOutcome::Confirmed)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_scope_does_not_admit_blocking_work() {
    let scope = ExecutionScope::new();
    let parent = register(&scope);
    parent.begin_dispatch().unwrap();
    scope
        .cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    let error = parent
        .context()
        .run_blocking(
            crate::ExecutionBudget::default(),
            |_| -> Result<crate::console::ConsoleResponse, crate::HostError> {
                panic!("stopped work must never execute")
            },
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::HostErrorCode::Cancelled);
    parent.complete(ExternalOutcome::Unknown, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    assert_eq!(scope.join().await.unwrap().operations().len(), 1);
}

#[test]
fn dropped_undispatched_registration_has_no_unfinished_external_work() {
    let scope = ExecutionScope::new();
    drop(register(&scope));
    scope.finish_body(true).unwrap();
    let report = scope.termination().unwrap().unwrap();
    assert_eq!(
        report.operations()[0].outcome(),
        Some(&ExternalOutcome::NotDispatched)
    );
}

#[test]
fn repeated_request_identity_does_not_deduplicate_occurrences() {
    let scope = ExecutionScope::new();
    let first = register(&scope);
    let second = register(&scope);
    assert_ne!(first.context().operation(), second.context().operation());
    first
        .complete(ExternalOutcome::NotDispatched, vec![])
        .unwrap();
    second
        .complete(ExternalOutcome::NotDispatched, vec![])
        .unwrap();
    scope.finish_body(true).unwrap();
    assert_eq!(scope.termination().unwrap().unwrap().operations().len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn stop_closes_admission_and_publishes_cause_before_notification() {
    let root = ExecutionScope::new();
    let child = root.child().unwrap();
    let signal = child.signal().unwrap();
    root.cancel_source()
        .stop(CancellationReason::Interrupt)
        .unwrap();
    let cause = signal.cancelled().await.unwrap();
    assert_eq!(cause.origin(), root.id());
    assert_eq!(cause.reason(), &CancellationReason::Interrupt);
    assert!(child.child().is_err());
    assert!(
        child
            .register(None, None, TraceContext::root(TraceId(1)))
            .is_err()
    );
    root.cancel_source()
        .stop(CancellationReason::Terminate)
        .unwrap();
    assert_eq!(signal.cause().unwrap(), Some(cause.clone()));
    child.finish_body(true).unwrap();
    root.finish_body(true).unwrap();
    let report = root.join().await.unwrap();
    assert_eq!(report.outcome(), &ScopeOutcome::Cancelled(cause));
    assert_eq!(report.causes().len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn child_stop_does_not_cancel_parent_or_sibling() {
    let root = ExecutionScope::new();
    let left = root.child().unwrap();
    let right = root.child().unwrap();
    left.cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    assert!(root.signal().unwrap().cause().unwrap().is_none());
    assert!(right.signal().unwrap().cause().unwrap().is_none());
    register(&right)
        .complete(ExternalOutcome::NotDispatched, vec![])
        .unwrap();
    left.finish_body(true).unwrap();
    right.finish_body(true).unwrap();
    root.finish_body(true).unwrap();
    assert_eq!(
        root.join().await.unwrap().outcome(),
        &ScopeOutcome::Completed
    );
}

#[tokio::test(flavor = "current_thread")]
async fn drain_waits_for_children_and_allows_their_owned_work() {
    let root = ExecutionScope::new();
    let child = root.child().unwrap();
    root.finish_body(true).unwrap();
    assert_eq!(root.state().unwrap(), ScopeState::Draining);
    assert!(root.child().is_err());
    let operation = register(&child);
    let nested = child
        .register(
            Some(operation.context().operation()),
            None,
            TraceContext::root(TraceId(1)),
        )
        .unwrap();
    child.finish_body(true).unwrap();
    assert!(root.termination().unwrap().is_none());
    nested
        .complete(ExternalOutcome::NotDispatched, vec![])
        .unwrap();
    operation
        .complete(ExternalOutcome::NotDispatched, vec![])
        .unwrap();
    assert_eq!(root.join().await.unwrap().operations().len(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn timeout_and_dropped_waiter_leave_work_owned_and_observable() {
    let root = ExecutionScope::new();
    let operation = register(&root);
    operation.begin_dispatch().unwrap();
    root.cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    root.finish_body(true).unwrap();
    let pending = root
        .wait_stopped(tokio::time::Instant::now())
        .await
        .unwrap();
    let StopWait::TimedOut(pending) = pending else {
        panic!("must retain work");
    };
    assert_eq!(pending.operations().len(), 1);
    assert!(
        tokio::time::timeout(Duration::ZERO, root.join())
            .await
            .is_err()
    );
    operation
        .complete(
            ExternalOutcome::Partial {
                completed_units: 12,
            },
            vec![],
        )
        .unwrap();
    let report = root.join().await.unwrap();
    assert_eq!(
        report.operations()[0].outcome(),
        Some(&ExternalOutcome::Partial {
            completed_units: 12
        })
    );
    // A late waiter sees a durable terminal result; stop cannot rewrite it.
    root.cancel_source()
        .stop(CancellationReason::Terminate)
        .unwrap();
    assert_eq!(root.join().await.unwrap().causes(), report.causes());
}

#[tokio::test(flavor = "current_thread")]
async fn dropping_registration_never_pretends_work_was_cancelled() {
    let root = ExecutionScope::new();
    let operation = register(&root);
    operation.begin_dispatch().unwrap();
    drop(operation);
    root.finish_body(true).unwrap();
    let StopWait::TimedOut(pending) = root
        .wait_stopped(tokio::time::Instant::now())
        .await
        .unwrap()
    else {
        panic!("lost owner must remain pending");
    };
    assert!(pending.operations()[0].owner_lost());
    assert!(pending.operations()[0].outcome().is_none());
}

#[test]
fn stop_between_registration_and_dispatch_rejects_dispatch() {
    let scope = ExecutionScope::new();
    let operation = register(&scope);
    scope
        .cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    assert!(operation.begin_dispatch().is_err());
    operation
        .complete(ExternalOutcome::NotDispatched, vec![])
        .unwrap();
    scope.finish_body(true).unwrap();
    assert!(scope.termination().unwrap().is_some());
}

#[test]
fn registration_stop_race_is_always_accounted_for() {
    for _ in 0..64 {
        let scope = ExecutionScope::new();
        let barrier = Arc::new(Barrier::new(2));
        let other = scope.clone();
        let ready = barrier.clone();
        let producer = std::thread::spawn(move || {
            ready.wait();
            other.register(None, None, TraceContext::root(TraceId(1)))
        });
        barrier.wait();
        scope
            .cancel_source()
            .stop(CancellationReason::Requested)
            .unwrap();
        if let Ok(operation) = producer.join().unwrap() {
            assert_eq!(scope.pending().unwrap().operations().len(), 1);
            assert!(operation.begin_dispatch().is_err());
            operation
                .complete(ExternalOutcome::NotDispatched, vec![])
                .unwrap();
        } else {
            assert!(scope.pending().unwrap().operations().is_empty());
        }
        scope.finish_body(true).unwrap();
        assert!(scope.termination().unwrap().is_some());
    }
}

#[test]
fn completion_and_stop_publish_one_immutable_outcome() {
    for _ in 0..64 {
        let scope = ExecutionScope::new();
        let barrier = Arc::new(Barrier::new(2));
        let other = scope.clone();
        let ready = barrier.clone();
        let finisher = std::thread::spawn(move || {
            ready.wait();
            other.finish_body(true).unwrap();
        });
        barrier.wait();
        scope
            .cancel_source()
            .stop(CancellationReason::Requested)
            .unwrap();
        finisher.join().unwrap();
        let report = scope.termination().unwrap().unwrap();
        assert!(matches!(
            report.outcome(),
            ScopeOutcome::Completed | ScopeOutcome::Cancelled(_)
        ));
        scope
            .cancel_source()
            .stop(CancellationReason::Terminate)
            .unwrap();
        assert_eq!(
            scope.termination().unwrap().unwrap().outcome(),
            report.outcome()
        );
    }
}
