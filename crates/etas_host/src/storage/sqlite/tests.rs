use super::*;
use crate::{HostErrorCode, HostRequestId, StorageLimits, TraceContext};
use crate::{
    TraceId,
    console::{ConsoleResponse, ConsoleResult},
    execution::{CancellationReason, ExecutionScope, ExternalOutcome, StopWait},
};
use rusqlite::Connection;
fn response() -> Result<ConsoleResponse, HostError> {
    Ok(ConsoleResponse {
        id: HostRequestId(1),
        result: ConsoleResult::Written,
    })
}
fn worker(limits: StorageLimits) -> SqliteWorker {
    SqliteWorker::new(Connection::open_in_memory().unwrap(), limits).unwrap()
}
#[tokio::test(flavor = "current_thread")]
async fn dropped_waiter_keeps_capacity_and_owned_database_work() {
    let worker = worker(StorageLimits {
        max_pending_jobs: 1,
        ..Default::default()
    });
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    parent.begin_dispatch().unwrap();
    let context = parent.context().clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let other = worker.clone();
    let waiter = tokio::spawn(async move {
        other.execute(Some(&context),HostRequestId(1),TraceContext::root(TraceId(1)),Default::default(),1,move |connection,_|{
            connection.execute_batch("CREATE TABLE evidence(value INTEGER); BEGIN IMMEDIATE; INSERT INTO evidence VALUES(7);").unwrap();
            started.send(()).unwrap();
            wait.recv().unwrap();
            connection.execute_batch("COMMIT").unwrap();
            response()
        }).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), ready)
        .await
        .unwrap()
        .unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    let error = worker
        .execute(
            None,
            HostRequestId(2),
            TraceContext::root(TraceId(1)),
            Default::default(),
            1,
            |_, _| response(),
        )
        .await
        .unwrap_err();
    let DispatchError::NotDispatched(error) = error else {
        panic!("saturation must prove non-dispatch")
    };
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
    assert!(error.message.contains("capacity"));
    parent.complete(ExternalOutcome::Confirmed, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    assert!(matches!(
        scope
            .wait_stopped(tokio::time::Instant::now())
            .await
            .unwrap(),
        StopWait::TimedOut(_)
    ));
    release.send(()).unwrap();
    let report = tokio::time::timeout(std::time::Duration::from_secs(2), scope.join())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        report.operations()[1].outcome(),
        Some(&ExternalOutcome::Confirmed)
    );
}
#[tokio::test(flavor = "current_thread")]
async fn cancelled_statement_rolls_back_and_next_job_is_not_interrupted() {
    let worker = worker(StorageLimits::default());
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    parent.begin_dispatch().unwrap();
    let context = parent.context().clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let other = worker.clone();
    let task = tokio::spawn(async move {
        other.execute(Some(&context),HostRequestId(1),TraceContext::root(TraceId(1)),Default::default(),1,move |connection,_|{
            connection.execute_batch("CREATE TABLE evidence(value INTEGER); BEGIN IMMEDIATE; INSERT INTO evidence VALUES(7);").unwrap();
            started.send(()).unwrap();
            connection.query_row("WITH RECURSIVE n(v) AS (VALUES(1) UNION ALL SELECT v+1 FROM n WHERE v<1000000000) SELECT sum(v) FROM n",[],|row|row.get::<_,i64>(0))
                .map_err(|_|HostError::new(HostErrorCode::Cancelled,"query cancelled"))?;
            response()
        }).await
    });
    ready.await.unwrap();
    scope
        .cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    let error = tokio::time::timeout(std::time::Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    let DispatchError::OutcomeUnavailable(error) = error else {
        panic!("started SQL must not be classified as never dispatched")
    };
    assert_eq!(error.code, HostErrorCode::Cancelled);
    parent.complete(ExternalOutcome::Unknown, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    scope.join().await.unwrap();
    worker
        .execute(
            None,
            HostRequestId(2),
            TraceContext::root(TraceId(2)),
            Default::default(),
            1,
            |connection, _| {
                assert!(connection.is_autocommit());
                let count: i64 = connection
                    .query_row("SELECT count(*) FROM evidence", [], |row| row.get(0))
                    .unwrap();
                assert_eq!(count, 0);
                connection
                    .execute_batch("INSERT INTO evidence VALUES(9)")
                    .unwrap();
                response()
            },
        )
        .await
        .unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn cancellation_while_queued_proves_domain_job_was_not_dispatched() {
    use std::future::Future;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let worker = worker(StorageLimits::default());
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let busy = worker.clone();
    let first = tokio::spawn(async move {
        busy.execute(
            None,
            HostRequestId(1),
            TraceContext::root(TraceId(1)),
            Default::default(),
            1,
            move |_, _| {
                started.send(()).unwrap();
                wait.recv().unwrap();
                response()
            },
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), ready)
        .await
        .unwrap()
        .unwrap();
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(2)), TraceContext::root(TraceId(1)))
        .unwrap();
    parent.begin_dispatch().unwrap();
    let context = parent.context().clone();
    let executed = Arc::new(AtomicBool::new(false));
    let mark = executed.clone();
    let queued = worker.execute(
        Some(&context),
        HostRequestId(2),
        TraceContext::root(TraceId(1)),
        Default::default(),
        1,
        move |_, _| {
            mark.store(true, Ordering::SeqCst);
            response()
        },
    );
    tokio::pin!(queued);
    // Poll through bounded admission and enqueue while the first owner is held.
    std::future::poll_fn(|cx| {
        assert!(queued.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    scope
        .cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    release.send(()).unwrap();
    first.await.unwrap().unwrap();
    let error = tokio::time::timeout(std::time::Duration::from_secs(2), queued)
        .await
        .unwrap()
        .unwrap_err();
    let DispatchError::NotDispatched(error) = error else {
        panic!("queued cancellation was classified as executed: {error:?}")
    };
    assert_eq!(error.code, HostErrorCode::Cancelled);
    assert!(!executed.load(Ordering::SeqCst));
    parent.complete(ExternalOutcome::Confirmed, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    let report = scope.join().await.unwrap();
    assert_eq!(report.operations().len(), 2);
    assert_eq!(
        report.operations()[1].outcome(),
        Some(&ExternalOutcome::NotDispatched)
    );
}

#[tokio::test(flavor = "current_thread")]
async fn lost_delivery_after_a_real_commit_is_not_reported_as_non_dispatch() {
    let workspace = crate::TestWorkspace::create("sqlite-lost-delivery").unwrap();
    let path = workspace.path().join("db");
    let limits = StorageLimits::default();
    let connection = open_durable(&path, &limits, |connection| {
        connection
            .execute_batch("CREATE TABLE evidence(value INTEGER)")
            .map_err(|e| HostError::new(HostErrorCode::ProviderUnavailable, e.to_string()))
    })
    .unwrap();
    let worker = SqliteWorker::new(connection, limits).unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        worker.execute::<ConsoleResponse, _>(
            None,
            HostRequestId(1),
            TraceContext::root(TraceId(1)),
            Default::default(),
            1,
            |connection, _| {
                connection
                    .execute_batch("BEGIN IMMEDIATE; INSERT INTO evidence VALUES(7); COMMIT;")
                    .unwrap();
                panic!("fault injection: response lost after commit");
            },
        ),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(DispatchError::OutcomeUnavailable(_))));
    let observer = Connection::open(path).unwrap();
    let value: i64 = observer
        .query_row("SELECT value FROM evidence", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        value, 7,
        "independent storage read must observe the committed write"
    );
}
