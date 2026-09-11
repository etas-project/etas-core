use etas_host::execution::{
    CancellationReason, ExecutionScope, ExternalOutcome, OperationRegistration, OperationResponse,
};
use etas_host::memory::*;
use etas_host::session::*;
use etas_host::*;

fn stopped() -> (ExecutionScope, OperationRegistration) {
    let scope = ExecutionScope::new();
    let parent = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    parent.begin_dispatch().unwrap();
    scope
        .cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    (scope, parent)
}
async fn finish(
    scope: ExecutionScope,
    parent: OperationRegistration,
    response: &impl OperationResponse,
    operation: &StorageOperationRef,
) {
    let evidence = response.external_outcome();
    assert_eq!(
        evidence,
        ExternalOutcome::StorageWrite(StorageWriteEvidence {
            operation: operation.clone(),
            status: CommitStatus::NotCommitted
        })
    );
    parent.complete(evidence, vec![]).unwrap();
    scope.finish_body(true).unwrap();
    let report = scope.join().await.unwrap();
    assert_eq!(
        report.operations().len(),
        1,
        "no database job admitted after cancellation"
    );
}
fn key() -> StorageOperationKey {
    StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap()
}

#[tokio::test(flavor = "current_thread")]
async fn sqlite_memory_pre_dispatch_cancellation_returns_bound_not_committed() {
    let workspace = TestWorkspace::create("memory-non-dispatch").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteMemoryClient::open(&path).unwrap();
    let store = StoreRef {
        region: MemoryRegionRef {
            stable_id: "dispatch".into(),
            schema_fingerprint: None,
        },
        path: vec![],
    };
    let mutation = MemoryMutation::Put {
        key: HostValue::String("key".into()),
        value: HostValue::String("private data".into()),
        condition: WriteCondition::Any,
    };
    let key = key();
    let operation = mutation
        .operation_ref(&store, key.clone(), &StorageLimits::default())
        .unwrap();
    let (scope, parent) = stopped();
    let response = client
        .write_scoped(
            MemoryWriteRequest {
                id: HostRequestId(1),
                store,
                operation: MemoryWriteOperation::Mutate { key, mutation },
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(1)),
                budget: Default::default(),
            },
            parent.context(),
        )
        .await
        .unwrap();
    assert!(
        matches!(&response.result,Ok(MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {operation:actual,reason:MemoryNotCommitted::Rejected(HostError {code:HostErrorCode::Cancelled,..})})) if actual==&operation)
    );
    finish(scope, parent, &response, &operation).await;
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM memory_entries", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM memory_receipts", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[tokio::test(flavor = "current_thread")]
async fn sqlite_session_pre_dispatch_cancellation_does_not_create_a_session() {
    let workspace = TestWorkspace::create("session-non-dispatch").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    let config = SessionConfig {
        id: "session".into(),
        context: ContextPolicy::All,
        retention: RetentionPolicy::Forever,
    };
    let key = key();
    let operation = resolve_operation_ref(&config, key.clone(), &StorageLimits::default()).unwrap();
    let (scope, parent) = stopped();
    let response = client
        .write_scoped(
            SessionWriteRequest {
                id: HostRequestId(1),
                operation: SessionWriteOperation::Resolve { key, config },
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(1)),
                budget: Default::default(),
            },
            parent.context(),
        )
        .await
        .unwrap();
    assert!(
        matches!(&response.result,Ok(SessionWriteResult::Outcome(WriteOutcome::NotCommitted {operation:actual,reason:HostError {code:HostErrorCode::Cancelled,..}})) if actual==&operation)
    );
    finish(scope, parent, &response, &operation).await;
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM sessions", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM session_receipts", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[tokio::test(flavor = "current_thread")]
async fn in_memory_pre_dispatch_cancellation_returns_bound_not_committed() {
    let client = InMemoryMemoryClient::new();
    let store = StoreRef {
        region: MemoryRegionRef {
            stable_id: "dispatch".into(),
            schema_fingerprint: None,
        },
        path: vec![],
    };
    let mutation = MemoryMutation::Put {
        key: HostValue::String("key".into()),
        value: HostValue::String("private data".into()),
        condition: WriteCondition::Any,
    };
    let key = key();
    let operation = mutation
        .operation_ref(&store, key.clone(), &StorageLimits::default())
        .unwrap();
    let (scope, parent) = stopped();
    let response = client
        .write_scoped(
            MemoryWriteRequest {
                id: HostRequestId(1),
                store: store.clone(),
                operation: MemoryWriteOperation::Mutate { key, mutation },
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(1)),
                budget: Default::default(),
            },
            parent.context(),
        )
        .await
        .unwrap();
    assert!(matches!(
        &response.result,
        Ok(MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: MemoryNotCommitted::Rejected(HostError {
                code: HostErrorCode::Cancelled,
                ..
            }),
            ..
        }))
    ));
    finish(scope, parent, &response, &operation).await;
    let value = client
        .execute(MemoryRequest {
            id: HostRequestId(2),
            store: store.clone(),
            operation: MemoryOperation::Get {
                key: HostValue::String("key".into()),
            },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
    assert!(matches!(value, MemoryResult::None));
    let receipt = client
        .write(MemoryWriteRequest {
            id: HostRequestId(3),
            store,
            operation: MemoryWriteOperation::Reconcile { operation },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
    assert!(matches!(
        receipt,
        MemoryWriteResult::Receipt(ReceiptLookup::Unresolved)
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn in_memory_session_pre_dispatch_cancellation_does_not_create_a_receipt() {
    let client = InMemorySessionClient::new();
    let config = SessionConfig {
        id: "session".into(),
        context: ContextPolicy::All,
        retention: RetentionPolicy::Forever,
    };
    let key = key();
    let operation = resolve_operation_ref(&config, key.clone(), &StorageLimits::default()).unwrap();
    let (scope, parent) = stopped();
    let response = client
        .write_scoped(
            SessionWriteRequest {
                id: HostRequestId(1),
                operation: SessionWriteOperation::Resolve { key, config },
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(1)),
                budget: Default::default(),
            },
            parent.context(),
        )
        .await
        .unwrap();
    assert!(matches!(
        &response.result,
        Ok(SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: HostError {
                code: HostErrorCode::Cancelled,
                ..
            },
            ..
        }))
    ));
    finish(scope, parent, &response, &operation).await;
    let receipt = client
        .write(SessionWriteRequest {
            id: HostRequestId(2),
            operation: SessionWriteOperation::Reconcile {
                session: SessionRef {
                    id: "session".into(),
                },
                operation,
            },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
    assert!(matches!(
        receipt,
        SessionWriteResult::Receipt(ReceiptLookup::Unresolved)
    ));
}
