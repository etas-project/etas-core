use etas_host::memory::*;
use etas_host::session::*;
use etas_host::*;
use std::future::Future;
use std::time::Duration;

// Occupy the actual blocking executor. A public client future must yield while
// its admitted work is queued rather than running synchronous storage inline.
async fn queued<T>(future: impl Future<Output = Result<T, HostError>>) -> T {
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let busy = tokio::task::spawn_blocking(move || {
        started.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
    });
    ready.await.unwrap();
    tokio::pin!(future);
    std::future::poll_fn(|cx| {
        assert!(
            future.as_mut().poll(cx).is_pending(),
            "public storage entry bypassed managed execution"
        );
        std::task::Poll::Ready(())
    })
    .await;
    release.send(()).unwrap();
    busy.await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), future)
        .await
        .unwrap()
        .unwrap()
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap()
}
fn store() -> StoreRef {
    StoreRef {
        region: MemoryRegionRef {
            stable_id: "public".into(),
            schema_fingerprint: None,
        },
        path: vec![],
    }
}
fn key() -> StorageOperationKey {
    StorageOperationKey::new(Duration::from_secs(60)).unwrap()
}
fn memory_request(operation: MemoryOperation) -> MemoryRequest {
    MemoryRequest {
        id: HostRequestId(1),
        store: store(),
        operation,
        authority: AuthorityContext::deny_all(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    }
}
fn session_request(operation: SessionOperation) -> SessionRequest {
    SessionRequest {
        id: HostRequestId(1),
        operation,
        authority: AuthorityContext::deny_all(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    }
}
fn config() -> SessionConfig {
    SessionConfig {
        id: "public".into(),
        context: ContextPolicy::All,
        retention: RetentionPolicy::Forever,
    }
}

#[test]
fn public_in_memory_reads_and_writes_use_the_managed_executor() {
    runtime().block_on(async {
        let memory = InMemoryMemoryClient::new();
        let put = queued(memory.execute(memory_request(MemoryOperation::Put {
            key: HostValue::String("k".into()),
            value: HostValue::Bool(true),
            condition: WriteCondition::Any,
        })))
        .await;
        assert!(matches!(put.result, Ok(MemoryResult::Written { .. })));
        let get = queued(memory.execute(memory_request(MemoryOperation::Get {
            key: HostValue::String("k".into()),
        })))
        .await;
        assert!(matches!(
            get.result,
            Ok(MemoryResult::Value {
                value: HostValue::Bool(true),
                ..
            })
        ));
        let write = queued(memory.write(MemoryWriteRequest {
            id: HostRequestId(2),
            store: store(),
            operation: MemoryWriteOperation::Mutate {
                key: key(),
                mutation: MemoryMutation::Delete {
                    key: HostValue::String("k".into()),
                    condition: WriteCondition::Exists,
                },
            },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        }))
        .await;
        assert!(matches!(
            write.result,
            Ok(MemoryWriteResult::Outcome(WriteOutcome::Committed(_)))
        ));

        let session = InMemorySessionClient::new();
        let resolve = queued(session.write(SessionWriteRequest {
            id: HostRequestId(3),
            operation: SessionWriteOperation::Resolve {
                key: key(),
                config: config(),
            },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        }))
        .await;
        assert!(matches!(
            resolve.result,
            Ok(SessionWriteResult::Outcome(WriteOutcome::Committed(_)))
        ));
        let loaded = queued(session.execute(session_request(SessionOperation::Load {
            session: SessionRef {
                id: "public".into(),
            },
            context: ContextPolicy::All,
            cursor: None,
            limit: Some(1),
        })))
        .await;
        let SessionResult::History {
            fence, messages, ..
        } = loaded.result.unwrap()
        else {
            panic!("history")
        };
        assert!(messages.is_empty());
        let intent = SessionRetentionIntent::prepare(
            SessionRef {
                id: "public".into(),
            },
            fence,
            -1,
            1,
            key(),
            &StorageLimits::default(),
        )
        .unwrap();
        let mut authority = AuthorityContext::deny_all();
        authority.grants.push(HostActionGrant::allow_with_args(
            "Memory",
            "write",
            vec![ActionArgPattern::Exact(HostValue::String("public".into()))],
        ));
        let maintenance = queued(session.maintain(SessionMaintenanceRequest {
            id: HostRequestId(4),
            operation: SessionMaintenanceOperation::Retain(Box::new(intent)),
            authority,
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        }))
        .await;
        assert!(matches!(
            maintenance.result,
            Ok(SessionMaintenanceResult::Outcome(
                WriteOutcome::NotCommitted {
                    reason: SessionRetentionRejection::NoChange(_),
                    ..
                }
            ))
        ));
    });
}

#[test]
fn missing_runtime_is_an_explicit_error_not_a_storage_panic() {
    let client = InMemoryMemoryClient::new();
    let mut future = client.execute(memory_request(MemoryOperation::Get {
        key: HostValue::String("k".into()),
    }));
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    let std::task::Poll::Ready(Err(error)) = future.as_mut().poll(&mut context) else {
        panic!("missing runtime must fail before dispatch")
    };
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
}

async fn result_limit<C: MemoryClient<Error = HostError>>(client: &C) {
    let inserted = client
        .execute(memory_request(MemoryOperation::Put {
            key: HostValue::String("oversized-result".into()),
            value: HostValue::Bytes(vec![7; 1024]),
            condition: WriteCondition::Any,
        }))
        .await
        .unwrap();
    assert!(matches!(inserted.result, Ok(MemoryResult::Written { .. })));
    let read = client
        .execute(memory_request(MemoryOperation::Get {
            key: HostValue::String("oversized-result".into()),
        }))
        .await
        .unwrap();
    assert_eq!(read.result.unwrap_err().code, HostErrorCode::BudgetExceeded);
    let page = client
        .execute(memory_request(MemoryOperation::Scan {
            cursor: None,
            limit: Some(1),
        }))
        .await
        .unwrap();
    assert_eq!(page.result.unwrap_err().code, HostErrorCode::BudgetExceeded);
}

#[tokio::test(flavor = "current_thread")]
async fn result_limits_apply_to_in_memory_and_sqlite_single_reads_and_pages() {
    let limits = StorageLimits {
        max_value_bytes: 4096,
        max_result_bytes: 256,
        ..Default::default()
    };
    result_limit(&InMemoryMemoryClient::with_limits(limits.clone()).unwrap()).await;
    let workspace = TestWorkspace::create("public-storage-result-limits").unwrap();
    result_limit(
        &SqliteMemoryClient::open_with_limits(workspace.path().join("db"), limits).unwrap(),
    )
    .await;
}
