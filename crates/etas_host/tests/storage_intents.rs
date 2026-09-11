use etas_host::{memory::*, *};

fn store() -> StoreRef {
    StoreRef {
        region: MemoryRegionRef {
            stable_id: "intent-test".into(),
            schema_fingerprint: Some("schema-v1".into()),
        },
        path: vec!["entries".into()],
    }
}
fn key() -> HostValue {
    HostValue::String("key".into())
}
fn prepare() -> MemoryWriteIntent {
    MemoryWriteIntent::prepare_put(
        store(),
        key(),
        HostValue::Bytes(vec![0, 255, 1]),
        WriteCondition::Missing,
        &StorageLimits::default(),
    )
    .unwrap()
}
fn request(intent: MemoryWriteIntent) -> MemoryWriteRequest {
    intent
        .into_request(
            HostRequestId(1),
            AuthorityContext::deny_all(),
            TraceContext::root(TraceId(1)),
            ExecutionBudget::default(),
            &StorageLimits::default(),
        )
        .unwrap()
}
async fn read<C: MemoryClient<Error = HostError>>(client: &C) -> MemoryResult {
    client
        .execute(MemoryRequest {
            id: HostRequestId(2),
            store: store(),
            operation: MemoryOperation::Get { key: key() },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap()
}
async fn commit<C: MemoryClient<Error = HostError>>(
    client: &C,
    intent: MemoryWriteIntent,
) -> MemoryWriteReceipt {
    let MemoryWriteResult::Outcome(WriteOutcome::Committed(receipt)) =
        client.write(request(intent)).await.unwrap().result.unwrap()
    else {
        panic!("expected committed intent");
    };
    receipt
}

#[tokio::test]
async fn prepare_is_read_only_and_restored_intent_replays_before_stale_condition() {
    async fn verify<C: MemoryClient<Error = HostError>>(client: C) {
        let intent = prepare();
        let encoded = intent.encode(&StorageLimits::default()).unwrap();
        let restored = MemoryWriteIntent::decode(&encoded, &StorageLimits::default()).unwrap();
        assert_eq!(restored, intent);
        assert_eq!(read(&client).await, MemoryResult::None);
        let mut lookup = request(intent.clone());
        lookup.operation = MemoryWriteOperation::Reconcile {
            operation: intent.operation_ref().clone(),
        };
        assert_eq!(
            client.write(lookup).await.unwrap().result.unwrap(),
            MemoryWriteResult::Receipt(ReceiptLookup::Unresolved)
        );
        let receipt = commit(&client, intent).await;
        assert_eq!(receipt.operation, *restored.operation_ref());
        assert_eq!(commit(&client, restored).await, receipt);
        assert!(
            matches!(read(&client).await, MemoryResult::Value { value: HostValue::Bytes(bytes), version } if bytes == [0, 255, 1] && matches!(&receipt.change, MemoryWriteChange::Written { version: written } if &version == written))
        );
    }
    verify(InMemoryMemoryClient::new()).await;
    let dir = TestWorkspace::create("intent-prepare").unwrap();
    verify(SqliteMemoryClient::open(dir.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn persisted_intent_survives_restart_and_delete_keeps_scoped_cas() {
    let dir = TestWorkspace::create("intent-restart").unwrap();
    let db = dir.path().join("db");
    let checkpoint = dir.path().join("intent");
    let intent = prepare();
    std::fs::write(
        &checkpoint,
        intent.encode(&StorageLimits::default()).unwrap(),
    )
    .unwrap();
    let receipt = {
        let client = SqliteMemoryClient::open(&db).unwrap();
        commit(&client, intent).await
    };
    let restored = MemoryWriteIntent::decode(
        &std::fs::read_to_string(&checkpoint).unwrap(),
        &StorageLimits::default(),
    )
    .unwrap();
    let client = SqliteMemoryClient::open(&db).unwrap();
    assert_eq!(commit(&client, restored).await, receipt);
    let deletion = MemoryWriteIntent::prepare_delete(
        store(),
        key(),
        WriteCondition::Match(receipt.change.revision().clone()),
        &StorageLimits::default(),
    )
    .unwrap();
    let restored = MemoryWriteIntent::decode(
        &deletion.encode(&StorageLimits::default()).unwrap(),
        &StorageLimits::default(),
    )
    .unwrap();
    let deleted = commit(&client, deletion).await;
    assert!(matches!(deleted.change, MemoryWriteChange::Deleted { .. }));
    assert_eq!(commit(&client, restored).await, deleted);
    assert_eq!(read(&client).await, MemoryResult::None);
}

#[tokio::test]
async fn reconciliation_restores_typed_target_and_rejects_a_changed_schema_binding() {
    async fn verify<C: MemoryClient<Error = HostError>>(client: C) {
        let key = HostValue::Record(vec![
            ("bytes".into(), HostValue::Bytes(vec![0, 255])),
            ("label".into(), HostValue::String("key".into())),
        ]);
        let intent = MemoryWriteIntent::prepare_put(
            store(),
            key.clone(),
            HostValue::Bool(true),
            WriteCondition::Any,
            &StorageLimits::default(),
        )
        .unwrap();
        let mut lookup = request(intent.clone());
        lookup.operation = MemoryWriteOperation::Reconcile {
            operation: intent.operation_ref().clone(),
        };
        let receipt = commit(&client, intent).await;
        assert_eq!(
            receipt.target,
            MemoryWriteTarget {
                store: store(),
                key
            }
        );
        assert_eq!(
            client.write(lookup.clone()).await.unwrap().result.unwrap(),
            MemoryWriteResult::Receipt(ReceiptLookup::Found(ConfirmedOutcome::Committed(receipt)))
        );
        lookup.store.region.schema_fingerprint = Some("different-schema".into());
        assert_eq!(
            client.write(lookup).await.unwrap().result.unwrap_err().code,
            HostErrorCode::InvalidRequest
        );
    }
    verify(InMemoryMemoryClient::new()).await;
    let dir = TestWorkspace::create("receipt-target").unwrap();
    verify(SqliteMemoryClient::open(dir.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn legacy_receipt_missing_target_remains_reserved_and_cannot_be_reexecuted() {
    let dir = TestWorkspace::create("legacy-receipt-target").unwrap();
    let db = dir.path().join("db");
    let intent = prepare();
    let receipt = {
        let client = SqliteMemoryClient::open(&db).unwrap();
        commit(&client, intent.clone()).await
    };
    {
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection
            .execute_batch(
                "ALTER TABLE memory_receipts DROP COLUMN key_json;
            ALTER TABLE memory_receipts DROP COLUMN schema_fingerprint;
            UPDATE etas_memory_schema SET format=2;",
            )
            .unwrap();
    }
    let client = SqliteMemoryClient::open(&db).unwrap();
    let mut lookup = request(intent.clone());
    lookup.operation = MemoryWriteOperation::Reconcile {
        operation: receipt.operation.clone(),
    };
    assert_eq!(
        client.write(lookup).await.unwrap().result.unwrap_err().code,
        HostErrorCode::SchemaMismatch
    );
    let result = client.write(request(intent)).await.unwrap().result.unwrap();
    assert!(
        matches!(result, MemoryWriteResult::Outcome(WriteOutcome::Unknown {
        operation, error: HostError { code: HostErrorCode::SchemaMismatch, .. }
    }) if operation == receipt.operation)
    );
    let connection = rusqlite::Connection::open(&db).unwrap();
    let (format, revision, count): (i64,i64,i64) = connection.query_row(
        "SELECT (SELECT format FROM etas_memory_schema), revision, (SELECT count(*) FROM memory_receipts) FROM memory_stores",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    assert_eq!((format, revision, count), (3, 1, 1));
}

#[test]
fn intent_codec_rejects_changed_target_payload_condition_format_and_extra_fields() {
    let limits = StorageLimits::default();
    let intent = MemoryWriteIntent::prepare_put(
        store(),
        key(),
        HostValue::String("private-payload".into()),
        WriteCondition::Missing,
        &limits,
    )
    .unwrap();
    let encoded = intent.encode(&limits).unwrap();
    for (from, to) in [
        ("intent-test", "other-store"),
        ("schema-v1", "schema-v2"),
        ("private-payload", "tampered-payload"),
        ("Missing", "Any"),
        ("etas.memory.write-intent.v1", "etas.memory.write-intent.v0"),
        ("\"fingerprint\"", "\"authority\""),
    ] {
        assert!(encoded.contains(from));
        let error = MemoryWriteIntent::decode(&encoded.replace(from, to), &limits).unwrap_err();
        assert!(!format!("{error:?}").contains("private-payload"));
    }
    assert!(!format!("{intent:?}").contains("private-payload"));
    assert!(!format!("{intent:?}").contains(intent.operation_ref().key.as_str()));
}

#[test]
fn restore_preserves_expired_identity_and_commit_uses_current_invocation() {
    let limits = StorageLimits::default();
    let intent = prepare();
    let original = intent.operation_ref().clone();
    let encoded = intent.encode(&limits).unwrap();
    let expired = format!("so1:{:016x}:{}", 1, &original.key.as_str()[21..]);
    let restored =
        MemoryWriteIntent::decode(&encoded.replace(original.key.as_str(), &expired), &limits)
            .unwrap();
    assert_eq!(restored.operation_ref().key.as_str(), expired);
    assert!(
        restored
            .into_request(
                HostRequestId(1),
                AuthorityContext::deny_all(),
                TraceContext::root(TraceId(1)),
                ExecutionBudget::default(),
                &limits
            )
            .is_err()
    );

    let authority = AuthorityContext::deny_all();
    let trace = TraceContext::root(TraceId(42));
    let request = intent
        .into_request(
            HostRequestId(99),
            authority.clone(),
            trace.clone(),
            ExecutionBudget::default(),
            &limits,
        )
        .unwrap();
    assert_eq!(request.authority, authority);
    assert_eq!(request.trace, trace);
    assert_eq!(request.id, HostRequestId(99));
    let MemoryWriteOperation::Mutate { key, .. } = request.operation else {
        panic!("intent mutation");
    };
    assert_eq!(key, original.key);
}

#[test]
fn intent_codec_applies_current_byte_depth_and_node_bounds() {
    let intent = prepare();
    let encoded = intent.encode(&StorageLimits::default()).unwrap();
    for limits in [
        StorageLimits {
            max_value_bytes: 100,
            ..StorageLimits::default()
        },
        StorageLimits {
            max_depth: 1,
            ..StorageLimits::default()
        },
        StorageLimits {
            max_nodes: 3,
            ..StorageLimits::default()
        },
    ] {
        assert!(MemoryWriteIntent::decode(&encoded, &limits).is_err());
        assert!(intent.encode(&limits).is_err());
    }
}

#[tokio::test]
async fn receipt_target_key_is_charged_before_mutation_on_both_backends() {
    async fn verify<C: MemoryClient<Error = HostError>>(client: C) {
        let large_key = HostValue::String("x".repeat(2048));
        let intent = MemoryWriteIntent::prepare_put(
            store(),
            large_key.clone(),
            HostValue::Bool(true),
            WriteCondition::Any,
            &StorageLimits::default(),
        )
        .unwrap();
        let result = client.write(request(intent)).await.unwrap().result.unwrap();
        assert!(matches!(
            result,
            MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
                reason: MemoryNotCommitted::Rejected(HostError {
                    code: HostErrorCode::BudgetExceeded,
                    ..
                }),
                ..
            })
        ));
        assert_eq!(
            client
                .execute(MemoryRequest {
                    id: HostRequestId(5),
                    store: store(),
                    operation: MemoryOperation::Get { key: large_key },
                    authority: AuthorityContext::deny_all(),
                    trace: TraceContext::root(TraceId(1)),
                    budget: ExecutionBudget::default(),
                })
                .await
                .unwrap()
                .result
                .unwrap(),
            MemoryResult::None
        );
    }
    let limits = StorageLimits {
        max_receipt_bytes: 2048,
        ..StorageLimits::default()
    };
    verify(InMemoryMemoryClient::with_limits(limits.clone()).unwrap()).await;
    let dir = TestWorkspace::create("receipt-key-budget").unwrap();
    verify(SqliteMemoryClient::open_with_limits(dir.path().join("db"), limits).unwrap()).await;
}

#[tokio::test]
async fn receipt_reconciliation_applies_current_decode_limits() {
    let dir = TestWorkspace::create("receipt-decode-limits").unwrap();
    let db = dir.path().join("db");
    let intent = MemoryWriteIntent::prepare_put(
        store(),
        HostValue::String("x".repeat(2048)),
        HostValue::Bool(true),
        WriteCondition::Any,
        &StorageLimits::default(),
    )
    .unwrap();
    let receipt = {
        let client = SqliteMemoryClient::open(&db).unwrap();
        commit(&client, intent.clone()).await
    };
    let limits = StorageLimits {
        max_result_bytes: 128,
        ..StorageLimits::default()
    };
    let client = SqliteMemoryClient::open_with_limits(&db, limits).unwrap();
    let mut lookup = request(intent);
    lookup.operation = MemoryWriteOperation::Reconcile {
        operation: receipt.operation,
    };
    assert!(client.write(lookup).await.unwrap().result.is_err());
}
