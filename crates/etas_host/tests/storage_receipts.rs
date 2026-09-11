use etas_host::memory::*;
use etas_host::*;

fn store() -> StoreRef {
    StoreRef {
        region: MemoryRegionRef {
            stable_id: "receipts".into(),
            schema_fingerprint: None,
        },
        path: vec![],
    }
}
fn put(value: &str) -> MemoryMutation {
    MemoryMutation::Put {
        key: HostValue::String("key".into()),
        value: HostValue::String(value.into()),
        condition: WriteCondition::Any,
    }
}
fn key() -> StorageOperationKey {
    StorageOperationKey::new(std::time::Duration::from_secs(3600)).unwrap()
}

#[test]
fn streaming_memory_fingerprint_preserves_v1_receipt_identity() {
    let store = StoreRef {
        region: MemoryRegionRef {
            stable_id: "region".into(),
            schema_fingerprint: Some("schema".into()),
        },
        path: vec!["records".into(), "nested".into()],
    };
    let key_json = r#"{"kind":"string","value":"key"}"#;
    let value_json =
        r#"{"fields":[{"name":"data","value":{"kind":"bytes","value":[0,255]}}],"kind":"record"}"#;
    let version = MemoryVersion::parse(&format!(
        "mv1:{}:{}:0000000000000007",
        "b".repeat(64),
        "a".repeat(32)
    ))
    .unwrap();
    for condition in [
        WriteCondition::Any,
        WriteCondition::Missing,
        WriteCondition::Exists,
        WriteCondition::Match(version),
    ] {
        for put in [true, false] {
            let mut hash = blake3::Hasher::new();
            hash.update(b"etas.memory.mutation.v1");
            hash.update(&[1]);
            hash.update(&2u64.to_le_bytes());
            let condition_text = match &condition {
                WriteCondition::Any => "any",
                WriteCondition::Missing => "missing",
                WriteCondition::Exists => "exists",
                WriteCondition::Match(v) => v.as_token(),
            };
            for part in [
                "region",
                "schema",
                "records",
                "nested",
                if put { "put" } else { "delete" },
                key_json,
                condition_text,
            ]
            .into_iter()
            .chain(put.then_some(value_json))
            {
                hash.update(&(part.len() as u64).to_le_bytes());
                hash.update(part.as_bytes());
            }
            let item = HostValue::String("key".into());
            let mutation = if put {
                MemoryMutation::Put {
                    key: item,
                    value: HostValue::Record(vec![("data".into(), HostValue::Bytes(vec![0, 255]))]),
                    condition: condition.clone(),
                }
            } else {
                MemoryMutation::Delete {
                    key: item,
                    condition: condition.clone(),
                }
            };
            assert_eq!(
                mutation
                    .operation_ref(&store, key(), &StorageLimits::default())
                    .unwrap()
                    .request_fingerprint,
                hash.finalize().to_hex().to_string()
            );
        }
    }
}
fn request(operation: MemoryWriteOperation) -> MemoryWriteRequest {
    MemoryWriteRequest {
        id: HostRequestId(1),
        store: store(),
        operation,
        authority: AuthorityContext::deny_all(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    }
}
async fn write<C: MemoryClient<Error = HostError>>(
    client: &C,
    operation: MemoryWriteOperation,
) -> MemoryWriteResult {
    client
        .write(request(operation))
        .await
        .unwrap()
        .result
        .unwrap()
}
fn committed(result: MemoryWriteResult) -> MemoryWriteReceipt {
    let MemoryWriteResult::Outcome(WriteOutcome::Committed(receipt)) = result else {
        panic!("expected committed receipt, got {result:?}")
    };
    receipt
}

#[test]
fn unknown_outcome_cannot_be_promoted_to_confirmed_receipt() {
    let operation = put("value")
        .operation_ref(&store(), key(), &StorageLimits::default())
        .unwrap();
    let unknown: MemoryWriteOutcome = WriteOutcome::Unknown {
        operation: operation.clone(),
        error: HostError::new(HostErrorCode::ProviderUnavailable, "lost acknowledgement"),
    };
    assert_eq!(
        MemoryConfirmedOutcome::try_from(unknown).unwrap_err().code,
        HostErrorCode::InvalidResponse
    );
    let rejection: MemoryWriteOutcome = WriteOutcome::NotCommitted {
        operation,
        reason: MemoryNotCommitted::Unchanged,
    };
    let confirmed = MemoryConfirmedOutcome::try_from(rejection.clone()).unwrap();
    assert_eq!(MemoryWriteOutcome::from(confirmed), rejection);
}
async fn verify<C: MemoryClient<Error = HostError>>(client: &C, durability: StorageDurability) {
    let key = key();
    let mutation = MemoryWriteOperation::Mutate {
        key: key.clone(),
        mutation: put("first"),
    };
    let receipt = committed(write(client, mutation.clone()).await);
    assert_eq!(receipt.durability, durability);
    assert_eq!(
        receipt.target,
        MemoryWriteTarget {
            store: store(),
            key: HostValue::String("key".into())
        }
    );
    assert!(matches!(receipt.change, MemoryWriteChange::Written { .. }));
    assert_eq!(
        committed(write(client, mutation).await),
        receipt,
        "same key must not allocate another version"
    );
    let different = client
        .write(request(MemoryWriteOperation::Mutate {
            key,
            mutation: put("other"),
        }))
        .await
        .unwrap()
        .result;
    match different {
        Err(error) => assert_eq!(error.code, HostErrorCode::InvalidRequest),
        Ok(MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: MemoryNotCommitted::Rejected(error),
            ..
        })) => assert_eq!(error.code, HostErrorCode::InvalidRequest),
        other => panic!("key reuse must reject: {other:?}"),
    }
    assert_eq!(
        write(
            client,
            MemoryWriteOperation::Reconcile {
                operation: receipt.operation.clone()
            }
        )
        .await,
        MemoryWriteResult::Receipt(ReceiptLookup::Found(ConfirmedOutcome::Committed(
            receipt.clone()
        )))
    );
    assert_eq!(
        write(
            client,
            MemoryWriteOperation::Reconcile {
                operation: StorageOperationRef {
                    key: self::key(),
                    request_fingerprint: receipt.operation.request_fingerprint.clone()
                }
            }
        )
        .await,
        MemoryWriteResult::Receipt(ReceiptLookup::Unresolved)
    );
    let conflict = MemoryWriteOperation::Mutate {
        key: self::key(),
        mutation: MemoryMutation::Put {
            key: HostValue::String("key".into()),
            value: HostValue::String("wrong".into()),
            condition: WriteCondition::Missing,
        },
    };
    let first = write(client, conflict.clone()).await;
    assert!(matches!(
        &first,
        MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: MemoryNotCommitted::Conflict { .. },
            ..
        })
    ));
    assert_eq!(write(client, conflict).await, first);
    let deletion = MemoryWriteOperation::Mutate {
        key: self::key(),
        mutation: MemoryMutation::Delete {
            key: HostValue::String("key".into()),
            condition: WriteCondition::Match(receipt.change.revision().clone()),
        },
    };
    let deleted = committed(write(client, deletion.clone()).await);
    assert!(matches!(deleted.change, MemoryWriteChange::Deleted { .. }));
    assert_eq!(deleted.target, receipt.target);
    assert_eq!(committed(write(client, deletion).await), deleted);
}

#[tokio::test]
async fn memory_write_receipts_replay_conditions_and_reconcile_on_both_backends() {
    verify(&InMemoryMemoryClient::new(), StorageDurability::Volatile).await;
    let workspace = TestWorkspace::create("memory-receipts").unwrap();
    verify(
        &SqliteMemoryClient::open(workspace.path().join("db")).unwrap(),
        StorageDurability::SqliteWalFull,
    )
    .await;
}

#[tokio::test]
async fn sqlite_lost_acknowledgement_reconciles_after_reopen_without_rewriting() {
    let workspace = TestWorkspace::create("memory-receipt-reopen").unwrap();
    let path = workspace.path().join("db");
    let key = key();
    let operation = put("committed")
        .operation_ref(&store(), key.clone(), &StorageLimits::default())
        .unwrap();
    let mutation = MemoryWriteOperation::Mutate {
        key,
        mutation: put("committed"),
    };
    let client = SqliteMemoryClient::open(&path).unwrap();
    drop(write(&client, mutation.clone()).await);
    drop(client);
    let independent = SqliteMemoryClient::open(&path).unwrap();
    let MemoryWriteResult::Receipt(ReceiptLookup::Found(ConfirmedOutcome::Committed(receipt))) =
        write(&independent, MemoryWriteOperation::Reconcile { operation }).await
    else {
        panic!("authoritative receipt must survive lost acknowledgement")
    };
    assert_eq!(committed(write(&independent, mutation).await), receipt);
    let connection = rusqlite::Connection::open(path).unwrap();
    let (revision, receipts): (i64, i64) = connection
        .query_row(
            "SELECT revision, (SELECT count(*) FROM memory_receipts) FROM memory_stores",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((revision, receipts), (1, 1));
}

async fn receipt_limits<C: MemoryClient<Error = HostError>>(client: &C) {
    let original = MemoryWriteOperation::Mutate {
        key: key(),
        mutation: put("first"),
    };
    let receipt = committed(write(client, original.clone()).await);
    let rejected = write(
        client,
        MemoryWriteOperation::Mutate {
            key: key(),
            mutation: put("second"),
        },
    )
    .await;
    assert!(matches!(
        rejected,
        MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: MemoryNotCommitted::Rejected(HostError {
                code: HostErrorCode::BudgetExceeded,
                ..
            }),
            ..
        })
    ));
    assert_eq!(
        committed(write(client, original).await),
        receipt,
        "capacity does not evict unexpired evidence"
    );
    let expired =
        StorageOperationKey::parse("so1:0000000000000001:00000000000000000000000000000001")
            .unwrap();
    assert!(
        client
            .write(request(MemoryWriteOperation::Mutate {
                key: expired.clone(),
                mutation: put("expired")
            }))
            .await
            .unwrap()
            .result
            .is_err()
    );
    assert_eq!(
        write(
            client,
            MemoryWriteOperation::Reconcile {
                operation: put("expired")
                    .operation_ref(&store(), expired, &StorageLimits::default())
                    .unwrap()
            }
        )
        .await,
        MemoryWriteResult::Receipt(ReceiptLookup::Expired)
    );
}

#[tokio::test]
async fn bounded_receipt_capacity_never_evicts_live_evidence_or_reexecutes_expired_keys() {
    let limits = StorageLimits {
        max_receipts: 1,
        ..Default::default()
    };
    receipt_limits(&InMemoryMemoryClient::with_limits(limits.clone()).unwrap()).await;
    let workspace = TestWorkspace::create("memory-receipt-limits").unwrap();
    receipt_limits(
        &SqliteMemoryClient::open_with_limits(workspace.path().join("db"), limits).unwrap(),
    )
    .await;
}

#[tokio::test]
async fn receipt_byte_budget_rejects_before_mutation_and_keeps_replay_evidence() {
    async fn verify<C: MemoryClient<Error = HostError>>(client: &C) {
        let mutation = MemoryWriteOperation::Mutate {
            key: key(),
            mutation: put("original"),
        };
        let original = committed(write(client, mutation.clone()).await);
        let rejected = write(
            client,
            MemoryWriteOperation::Mutate {
                key: key(),
                mutation: put("must not overwrite"),
            },
        )
        .await;
        assert!(matches!(
            rejected,
            MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
                reason: MemoryNotCommitted::Rejected(HostError {
                    code: HostErrorCode::BudgetExceeded,
                    ..
                }),
                ..
            })
        ));
        assert_eq!(committed(write(client, mutation).await), original);
        let value = client
            .execute(MemoryRequest {
                id: HostRequestId(4),
                store: store(),
                operation: MemoryOperation::Get {
                    key: HostValue::String("key".into()),
                },
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(1)),
                budget: ExecutionBudget::default(),
            })
            .await
            .unwrap()
            .result
            .unwrap();
        assert_eq!(
            value,
            MemoryResult::Value {
                value: HostValue::String("original".into()),
                version: original.change.revision().clone()
            }
        );
        assert_eq!(
            write(
                client,
                MemoryWriteOperation::Reconcile {
                    operation: original.operation.clone()
                }
            )
            .await,
            MemoryWriteResult::Receipt(ReceiptLookup::Found(ConfirmedOutcome::Committed(original)))
        );
    }
    let limits = StorageLimits {
        max_receipt_bytes: 1800,
        ..Default::default()
    };
    verify(&InMemoryMemoryClient::with_limits(limits.clone()).unwrap()).await;
    let workspace = TestWorkspace::create("memory-receipt-byte-budget").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteMemoryClient::open_with_limits(&path, limits.clone()).unwrap();
    verify(&client).await;
    drop(client);
    let reopened = SqliteMemoryClient::open_with_limits(path, limits).unwrap();
    assert!(matches!(
        write(
            &reopened,
            MemoryWriteOperation::Mutate {
                key: key(),
                mutation: put("reopen must not reset quota")
            }
        )
        .await,
        MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: MemoryNotCommitted::Rejected(HostError {
                code: HostErrorCode::BudgetExceeded,
                ..
            }),
            ..
        })
    ));
}

#[tokio::test]
async fn sqlite_concurrent_same_operation_has_one_revision_and_receipt() {
    let workspace = TestWorkspace::create("memory-receipt-race").unwrap();
    let path = workspace.path().join("db");
    let left = SqliteMemoryClient::open(&path).unwrap();
    let right = SqliteMemoryClient::open(&path).unwrap();
    let operation = MemoryWriteOperation::Mutate {
        key: key(),
        mutation: put("once"),
    };
    let (a, b) = tokio::join!(write(&left, operation.clone()), write(&right, operation));
    assert_eq!(committed(a), committed(b));
    let connection = rusqlite::Connection::open(path).unwrap();
    let revision: i64 = connection
        .query_row("SELECT revision FROM memory_stores", [], |r| r.get(0))
        .unwrap();
    assert_eq!(revision, 1);
}

#[tokio::test]
async fn sqlite_receipt_failure_rolls_back_mutation_and_revision() {
    let workspace = TestWorkspace::create("memory-receipt-rollback").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteMemoryClient::open(&path).unwrap();
    let connection = rusqlite::Connection::open(path).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON memory_receipts BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END;").unwrap();
    let result = write(
        &client,
        MemoryWriteOperation::Mutate {
            key: key(),
            mutation: put("must not commit"),
        },
    )
    .await;
    assert!(matches!(
        result,
        MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: MemoryNotCommitted::Rejected(_),
            ..
        })
    ));
    let entries: i64 = connection
        .query_row("SELECT COUNT(*) FROM memory_entries", [], |r| r.get(0))
        .unwrap();
    let stores: i64 = connection
        .query_row("SELECT COUNT(*) FROM memory_stores", [], |r| r.get(0))
        .unwrap();
    assert_eq!((entries, stores), (0, 0));
}
