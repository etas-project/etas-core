use etas_host::session::*;
use etas_host::*;

fn session() -> SessionRef {
    SessionRef {
        id: "receipts".into(),
    }
}
fn message(id: &str) -> SessionMessage {
    SessionMessage {
        id: id.into(),
        session: session(),
        role: SessionMessageRole::User,
        from: None,
        to: None,
        created_at: "0".into(),
        payload: HostValue::String("hello".into()),
        provenance: None,
        dedup_key: Some("logical-append".into()),
    }
}
fn key() -> StorageOperationKey {
    StorageOperationKey::new(std::time::Duration::from_secs(3600)).unwrap()
}

#[test]
fn streaming_session_fingerprint_preserves_v1_receipt_identity() {
    let wire = concat!(
        r#"{"fields":[{"name":"id","value":{"kind":"string","value":"m1"}},"#,
        r#"{"name":"from","value":{"fields":[],"kind":"variant","name":"None"}},"#,
        r#"{"name":"to","value":{"fields":[],"kind":"variant","name":"None"}},"#,
        r#"{"name":"role","value":{"kind":"string","value":"user"}},"#,
        r#"{"name":"session","value":{"fields":[{"kind":"string","value":"receipts"}],"kind":"variant","name":"Some"}},"#,
        r#"{"name":"created_at","value":{"kind":"string","value":"0"}},"#,
        r#"{"name":"payload","value":{"kind":"string","value":"hello"}},"#,
        r#"{"name":"provenance","value":{"fields":[],"kind":"variant","name":"None"}},"#,
        r#"{"name":"dedup_key","value":{"fields":[{"kind":"string","value":"logical-append"}],"kind":"variant","name":"Some"}}],"kind":"record"}"#,
    );
    let mut hash = blake3::Hasher::new_derive_key("etas.session.append.v1");
    hash.update(wire.as_bytes());
    assert_eq!(
        append_operation_ref(&message("m1"), key(), &StorageLimits::default())
            .unwrap()
            .request_fingerprint,
        hash.finalize().to_hex().to_string()
    );
}
fn request(operation: SessionWriteOperation) -> SessionWriteRequest {
    SessionWriteRequest {
        id: HostRequestId(1),
        operation,
        authority: AuthorityContext::deny_all(),
        trace: TraceContext::root(TraceId(1)),
        budget: ExecutionBudget::default(),
    }
}
async fn initialize<C: SessionClient<Error = HostError>>(client: &C) {
    client
        .execute(SessionRequest {
            id: HostRequestId(0),
            operation: SessionOperation::Resolve {
                config: SessionConfig {
                    id: session().id,
                    context: ContextPolicy::All,
                    retention: RetentionPolicy::Forever,
                },
            },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
}
async fn write<C: SessionClient<Error = HostError>>(
    client: &C,
    operation: SessionWriteOperation,
) -> SessionWriteResult {
    client
        .write(request(operation))
        .await
        .unwrap()
        .result
        .unwrap()
}
fn committed(result: SessionWriteResult) -> SessionAppendReceipt {
    let SessionWriteResult::Outcome(WriteOutcome::Committed(SessionWriteReceipt::Append(receipt))) =
        result
    else {
        panic!("expected committed receipt, got {result:?}");
    };
    receipt
}
fn path(name: &str) -> std::path::PathBuf {
    let mut nonce = [0; 16];
    getrandom::fill(&mut nonce).unwrap();
    std::env::temp_dir().join(format!(
        "etas-session-receipts-{name}-{:x}.db",
        u128::from_be_bytes(nonce)
    ))
}

#[tokio::test]
async fn receipt_byte_budget_rolls_back_append_and_keeps_prior_receipt() {
    async fn verify<C: SessionClient<Error = HostError>>(client: &C) {
        initialize(client).await;
        let first = message(&"\u{6587}".repeat(100));
        let mutation = SessionWriteOperation::Append {
            key: key(),
            message: first.clone().into(),
        };
        let original = committed(write(client, mutation.clone()).await);
        let mut second = message("not-committed");
        second.dedup_key = None;
        let result = write(
            client,
            SessionWriteOperation::Append {
                key: key(),
                message: second.into(),
            },
        )
        .await;
        assert!(matches!(
            result,
            SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                reason: HostError {
                    code: HostErrorCode::BudgetExceeded,
                    ..
                },
                ..
            })
        ));
        assert_eq!(committed(write(client, mutation).await), original);
        assert_eq!(
            write(
                client,
                SessionWriteOperation::Reconcile {
                    session: session(),
                    operation: original.operation.clone()
                }
            )
            .await,
            SessionWriteResult::Receipt(ReceiptLookup::Found(SessionWriteReceipt::Append(
                original
            )))
        );
        let result = client
            .execute(SessionRequest {
                id: HostRequestId(5),
                operation: SessionOperation::Load {
                    session: session(),
                    context: ContextPolicy::All,
                    cursor: None,
                    limit: Some(10),
                },
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(1)),
                budget: ExecutionBudget::default(),
            })
            .await
            .unwrap()
            .result
            .unwrap();
        let SessionResult::History { messages, .. } = result else {
            panic!("history")
        };
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, first.id);
    }
    let limits = StorageLimits {
        max_receipt_bytes: 2500,
        ..Default::default()
    };
    verify(&InMemorySessionClient::with_limits(limits.clone()).unwrap()).await;
    let workspace = TestWorkspace::create("session-receipt-byte-budget").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open_with_limits(&path, limits.clone()).unwrap();
    verify(&client).await;
    drop(client);
    let reopened = SqliteSessionClient::open_with_limits(path, limits).unwrap();
    let mut next = message("after-reopen");
    next.dedup_key = None;
    assert!(matches!(
        write(
            &reopened,
            SessionWriteOperation::Append {
                key: key(),
                message: next.into()
            }
        )
        .await,
        SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: HostError {
                code: HostErrorCode::BudgetExceeded,
                ..
            },
            ..
        })
    ));
}

#[tokio::test]
async fn append_identity_and_dedup_preserve_receipt_version() {
    async fn verify<C: SessionClient<Error = HostError>>(client: C, durability: StorageDurability) {
        initialize(&client).await;
        let mutation = SessionWriteOperation::Append {
            key: key(),
            message: message("first").into(),
        };
        let first = committed(write(&client, mutation.clone()).await);
        assert!(first.version.belongs_to(&session().id));
        assert!(!first.version.belongs_to("different-session"));
        assert_eq!(first.durability, durability);
        assert!(!first.deduplicated);
        assert_eq!(committed(write(&client, mutation).await), first);
        assert_eq!(
            write(
                &client,
                SessionWriteOperation::Reconcile {
                    session: session(),
                    operation: first.operation.clone()
                }
            )
            .await,
            SessionWriteResult::Receipt(ReceiptLookup::Found(SessionWriteReceipt::Append(
                first.clone()
            )))
        );
        let dedup = committed(
            write(
                &client,
                SessionWriteOperation::Append {
                    key: key(),
                    message: message("different-delivery").into(),
                },
            )
            .await,
        );
        assert_eq!(dedup.version, first.version);
        assert_eq!(dedup.message_id, "first");
        assert!(dedup.deduplicated);
        let mut changed = message("first");
        changed.payload = HostValue::String("different".into());
        let mismatch = client
            .write(request(SessionWriteOperation::Append {
                key: first.operation.key.clone(),
                message: changed.into(),
            }))
            .await
            .unwrap()
            .result;
        assert!(matches!(
            mismatch,
            Ok(SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                reason: HostError {
                    code: HostErrorCode::InvalidRequest,
                    ..
                },
                ..
            }))
        ));
        assert_eq!(
            write(
                &client,
                SessionWriteOperation::Reconcile {
                    session: session(),
                    operation: first.operation.clone()
                }
            )
            .await,
            SessionWriteResult::Receipt(ReceiptLookup::Found(SessionWriteReceipt::Append(
                first.clone()
            )))
        );
    }
    verify(InMemorySessionClient::new(), StorageDurability::Volatile).await;
    verify(
        SqliteSessionClient::open(path("identity")).unwrap(),
        StorageDurability::SqliteWalFull,
    )
    .await;
}

#[tokio::test]
async fn sqlite_lost_append_acknowledgement_reconciles_after_reopen() {
    let path = path("lost-ack");
    let client = SqliteSessionClient::open(&path).unwrap();
    initialize(&client).await;
    let key = key();
    let original = message("first");
    let operation =
        append_operation_ref(&original, key.clone(), &StorageLimits::default()).unwrap();
    // Execute the actual transaction and deliberately lose the delivery.
    drop(
        client
            .write(request(SessionWriteOperation::Append {
                key: key.clone(),
                message: original.clone().into(),
            }))
            .await
            .unwrap(),
    );
    drop(client);
    let restored = SqliteSessionClient::open(&path).unwrap();
    let SessionWriteResult::Receipt(ReceiptLookup::Found(receipt)) = write(
        &restored,
        SessionWriteOperation::Reconcile {
            session: session(),
            operation,
        },
    )
    .await
    else {
        panic!("receipt");
    };
    let repeated = committed(
        write(
            &restored,
            SessionWriteOperation::Append {
                key,
                message: original.into(),
            },
        )
        .await,
    );
    assert_eq!(receipt, SessionWriteReceipt::Append(repeated));
    let connection = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM session_messages", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT last_ordinal FROM session_sequences", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn sqlite_same_operation_concurrent_connections_append_once() {
    let path = path("concurrent");
    let first = SqliteSessionClient::open(&path).unwrap();
    initialize(&first).await;
    let second = SqliteSessionClient::open(&path).unwrap();
    let op = SessionWriteOperation::Append {
        key: key(),
        message: message("first").into(),
    };
    let (a, b) = tokio::join!(write(&first, op.clone()), write(&second, op));
    assert_eq!(committed(a), committed(b));
    let connection = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM session_receipts", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM session_messages", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn sqlite_receipt_failure_rolls_back_append_and_ordinal() {
    let path = path("rollback");
    let client = SqliteSessionClient::open(&path).unwrap();
    initialize(&client).await;
    let connection = rusqlite::Connection::open(path).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON session_receipts BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END;").unwrap();
    assert!(matches!(
        write(
            &client,
            SessionWriteOperation::Append {
                key: key(),
                message: message("first").into()
            }
        )
        .await,
        SessionWriteResult::Outcome(WriteOutcome::NotCommitted { .. })
    ));
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM session_messages", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        connection
            .query_row("SELECT last_ordinal FROM session_sequences", [], |row| row
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        -1
    );
}

#[tokio::test]
async fn receipt_capacity_preserves_live_evidence_and_expiry_never_reexecutes() {
    async fn verify<C: SessionClient<Error = HostError>>(client: C) {
        initialize(&client).await;
        let first = committed(
            write(
                &client,
                SessionWriteOperation::Append {
                    key: key(),
                    message: message("first").into(),
                },
            )
            .await,
        );
        let mut second = message("second");
        second.dedup_key = None;
        assert!(matches!(
            write(
                &client,
                SessionWriteOperation::Append {
                    key: key(),
                    message: second.into()
                }
            )
            .await,
            SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                reason: HostError {
                    code: HostErrorCode::BudgetExceeded,
                    ..
                },
                ..
            })
        ));
        assert_eq!(
            write(
                &client,
                SessionWriteOperation::Reconcile {
                    session: session(),
                    operation: first.operation.clone()
                }
            )
            .await,
            SessionWriteResult::Receipt(ReceiptLookup::Found(SessionWriteReceipt::Append(first)))
        );
        let expired =
            StorageOperationKey::parse("so1:0000000000000001:00000000000000000000000000000001")
                .unwrap();
        let reference = append_operation_ref(
            &message("expired"),
            expired.clone(),
            &StorageLimits::default(),
        )
        .unwrap();
        assert!(matches!(
            write(
                &client,
                SessionWriteOperation::Append {
                    key: expired,
                    message: message("expired").into()
                }
            )
            .await,
            SessionWriteResult::Outcome(WriteOutcome::NotCommitted { .. })
        ));
        assert_eq!(
            write(
                &client,
                SessionWriteOperation::Reconcile {
                    session: session(),
                    operation: reference
                }
            )
            .await,
            SessionWriteResult::Receipt(ReceiptLookup::Expired)
        );
    }
    let limits = StorageLimits {
        max_receipts: 1,
        ..StorageLimits::default()
    };
    verify(InMemorySessionClient::with_limits(limits.clone()).unwrap()).await;
    verify(SqliteSessionClient::open_with_limits(path("capacity"), limits).unwrap()).await;
}
