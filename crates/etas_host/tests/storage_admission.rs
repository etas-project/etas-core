use etas_host::execution::{ExecutionScope, ExternalOutcome};
use etas_host::memory::*;
use etas_host::session::*;
use etas_host::*;
use std::{future::Future, time::Duration};

async fn occupy_pool() -> (std::sync::mpsc::Sender<()>, tokio::task::JoinHandle<()>) {
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let busy = tokio::task::spawn_blocking(move || {
        started.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(10)).unwrap();
    });
    ready.await.unwrap();
    (release, busy)
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap()
}
fn limits(by_bytes: bool) -> StorageLimits {
    StorageLimits {
        max_value_bytes: 4096,
        max_result_bytes: 4096,
        max_pending_bytes: if by_bytes { 12_000 } else { 1_000_000 },
        max_pending_jobs: if by_bytes { 32 } else { 1 },
        ..Default::default()
    }
}
async fn poll_queued<T>(future: std::pin::Pin<&mut impl Future<Output = T>>) {
    let mut future = future;
    std::future::poll_fn(|cx| {
        assert!(
            future.as_mut().poll(cx).is_pending(),
            "request was not admitted to the occupied executor"
        );
        std::task::Poll::Ready(())
    })
    .await;
}
fn store() -> StoreRef {
    StoreRef {
        region: MemoryRegionRef {
            stable_id: "admission".into(),
            schema_fingerprint: None,
        },
        path: vec![],
    }
}
fn trace() -> TraceContext {
    TraceContext::root(TraceId(1))
}

async fn memory_admission(limits: StorageLimits) {
    let client = InMemoryMemoryClient::with_limits(limits.clone()).unwrap();
    let shared = client.clone();
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(1)), trace())
        .unwrap();
    parent.begin_dispatch().unwrap();
    let context = parent.context().clone();
    let (release, busy) = occupy_pool().await;
    let mut first = Box::pin(client.execute_scoped(
        MemoryRequest {
            id: HostRequestId(1),
            store: store(),
            operation: MemoryOperation::Get {
                key: HostValue::Bool(false),
            },
            authority: AuthorityContext::deny_all(),
            trace: trace(),
            budget: Default::default(),
        },
        &context,
    ));
    poll_queued(first.as_mut()).await;
    drop(first);
    let key = StorageOperationKey::new(Duration::from_secs(60)).unwrap();
    let mutation = MemoryMutation::Put {
        key: HostValue::Bool(true),
        value: HostValue::Bool(true),
        condition: WriteCondition::Any,
    };
    let operation = mutation
        .operation_ref(&store(), key.clone(), &limits)
        .unwrap();
    let request = MemoryWriteRequest {
        id: HostRequestId(2),
        store: store(),
        operation: MemoryWriteOperation::Mutate { key, mutation },
        authority: AuthorityContext::deny_all(),
        trace: trace(),
        budget: Default::default(),
    };
    let rejected = tokio::time::timeout(Duration::from_secs(1), shared.write(request.clone()))
        .await
        .unwrap()
        .unwrap()
        .result
        .unwrap();
    assert!(
        matches!(rejected, MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
        operation: ref actual, reason: MemoryNotCommitted::Rejected(HostError { code: HostErrorCode::ProviderUnavailable, .. }),
    }) if actual == &operation)
    );
    parent.complete(ExternalOutcome::Unknown, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    release.send(()).unwrap();
    busy.await.unwrap();
    let report = tokio::time::timeout(Duration::from_secs(2), scope.join())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(report.operations().len(), 2);
    assert_eq!(
        report.operations()[1].outcome(),
        Some(&ExternalOutcome::Confirmed)
    );
    assert!(matches!(
        shared.write(request).await.unwrap().result.unwrap(),
        MemoryWriteResult::Outcome(WriteOutcome::Committed(_))
    ));
}

async fn session_admission(limits: StorageLimits) {
    let client = InMemorySessionClient::with_limits(limits.clone()).unwrap();
    let shared = client.clone();
    let config = SessionConfig {
        id: "admission".into(),
        context: ContextPolicy::All,
        retention: RetentionPolicy::Forever,
    };
    client
        .execute(SessionRequest {
            id: HostRequestId(0),
            operation: SessionOperation::Resolve {
                config: config.clone(),
            },
            authority: AuthorityContext::deny_all(),
            trace: trace(),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(1)), trace())
        .unwrap();
    parent.begin_dispatch().unwrap();
    let context = parent.context().clone();
    let (release, busy) = occupy_pool().await;
    let mut first = Box::pin(client.execute_scoped(
        SessionRequest {
            id: HostRequestId(1),
            operation: SessionOperation::Load {
                session: SessionRef {
                    id: config.id.clone(),
                },
                context: ContextPolicy::All,
                cursor: None,
                limit: Some(1),
            },
            authority: AuthorityContext::deny_all(),
            trace: trace(),
            budget: Default::default(),
        },
        &context,
    ));
    poll_queued(first.as_mut()).await;
    drop(first);
    let key = StorageOperationKey::new(Duration::from_secs(60)).unwrap();
    let operation = resolve_operation_ref(&config, key.clone(), &limits).unwrap();
    let request = SessionWriteRequest {
        id: HostRequestId(2),
        operation: SessionWriteOperation::Resolve { key, config },
        authority: AuthorityContext::deny_all(),
        trace: trace(),
        budget: Default::default(),
    };
    let rejected = tokio::time::timeout(Duration::from_secs(1), shared.write(request.clone()))
        .await
        .unwrap()
        .unwrap()
        .result
        .unwrap();
    assert!(
        matches!(rejected, SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
        operation: ref actual, reason: HostError { code: HostErrorCode::ProviderUnavailable, .. },
    }) if actual == &operation)
    );
    parent.complete(ExternalOutcome::Unknown, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    release.send(()).unwrap();
    busy.await.unwrap();
    let report = tokio::time::timeout(Duration::from_secs(2), scope.join())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(report.operations().len(), 2);
    assert_eq!(
        report.operations()[1].outcome(),
        Some(&ExternalOutcome::Confirmed)
    );
    assert!(matches!(
        shared.write(request).await.unwrap().result.unwrap(),
        SessionWriteResult::Outcome(WriteOutcome::Committed(_))
    ));
}

#[test]
fn volatile_job_limit_is_shared_by_clones_and_survives_waiter_drop() {
    runtime().block_on(async {
        memory_admission(limits(false)).await;
        session_admission(limits(false)).await;
    });
}
#[test]
fn volatile_byte_limit_is_shared_by_clones_and_survives_waiter_drop() {
    runtime().block_on(async {
        memory_admission(limits(true)).await;
        session_admission(limits(true)).await;
    });
}
