use etas_host::{session::*, *};

fn config() -> SessionConfig {
    SessionConfig {
        id: "resolve-receipt".into(),
        context: ContextPolicy::All,
        retention: RetentionPolicy::Forever,
    }
}
fn key() -> StorageOperationKey {
    StorageOperationKey::new(std::time::Duration::from_secs(3600)).unwrap()
}
async fn write<C: SessionClient<Error = HostError>>(
    client: &C,
    operation: SessionWriteOperation,
) -> SessionWriteResult {
    client
        .write(SessionWriteRequest {
            id: HostRequestId(1),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap()
}
fn committed(result: SessionWriteResult) -> SessionResolveReceipt {
    let SessionWriteResult::Outcome(WriteOutcome::Committed(SessionWriteReceipt::Resolve(receipt))) =
        result
    else {
        panic!("resolve receipt: {result:?}")
    };
    receipt
}

#[tokio::test]
async fn resolve_replay_preserves_atomic_creation_result_and_configuration_binding() {
    async fn verify<C: SessionClient<Error = HostError>>(client: C) {
        let config = config();
        let operation = SessionWriteOperation::Resolve {
            key: key(),
            config: config.clone(),
        };
        let first = committed(write(&client, operation.clone()).await);
        assert!(first.created);
        assert_eq!(committed(write(&client, operation).await), first);
        let second = committed(
            write(
                &client,
                SessionWriteOperation::Resolve {
                    key: key(),
                    config: config.clone(),
                },
            )
            .await,
        );
        assert!(!second.created);
        assert_eq!(second.generation, first.generation);
        let mut changed = config.clone();
        changed.context = ContextPolicy::LastTurns(4);
        assert!(matches!(
            write(
                &client,
                SessionWriteOperation::Resolve {
                    key: first.operation.key.clone(),
                    config: changed.clone()
                }
            )
            .await,
            SessionWriteResult::Outcome(WriteOutcome::NotCommitted { .. })
        ));
        assert!(matches!(
            write(
                &client,
                SessionWriteOperation::Resolve {
                    key: key(),
                    config: changed
                }
            )
            .await,
            SessionWriteResult::Outcome(WriteOutcome::NotCommitted { .. })
        ));
        assert_eq!(
            write(
                &client,
                SessionWriteOperation::Reconcile {
                    session: first.session.clone(),
                    operation: first.operation.clone()
                }
            )
            .await,
            SessionWriteResult::Receipt(ReceiptLookup::Found(SessionWriteReceipt::Resolve(first)))
        );
    }
    verify(InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("session-resolve-replay").unwrap();
    verify(SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn lost_resolve_acknowledgement_recovers_original_receipt_after_reopen() {
    let workspace = TestWorkspace::create("session-resolve-lost-ack").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    let key = key();
    let reference =
        resolve_operation_ref(&config(), key.clone(), &StorageLimits::default()).unwrap();
    drop(
        write(
            &client,
            SessionWriteOperation::Resolve {
                key: key.clone(),
                config: config(),
            },
        )
        .await,
    );
    drop(client);
    let client = SqliteSessionClient::open(&path).unwrap();
    let SessionWriteResult::Receipt(ReceiptLookup::Found(SessionWriteReceipt::Resolve(receipt))) =
        write(
            &client,
            SessionWriteOperation::Reconcile {
                session: SessionRef { id: config().id },
                operation: reference,
            },
        )
        .await
    else {
        panic!("receipt lookup")
    };
    assert!(receipt.created);
    assert_eq!(
        committed(
            write(
                &client,
                SessionWriteOperation::Resolve {
                    key,
                    config: config()
                }
            )
            .await
        ),
        receipt
    );
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM sessions", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM session_receipts", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn resolve_receipt_failure_rolls_back_session_and_generation() {
    let workspace = TestWorkspace::create("session-resolve-receipt-failure").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    let db = rusqlite::Connection::open(path).unwrap();
    db.execute_batch("CREATE TRIGGER reject_resolve_receipt BEFORE INSERT ON session_receipts BEGIN SELECT RAISE(ABORT,'injected receipt error'); END;").unwrap();
    assert!(matches!(
        write(
            &client,
            SessionWriteOperation::Resolve {
                key: key(),
                config: config()
            }
        )
        .await,
        SessionWriteResult::Outcome(WriteOutcome::NotCommitted { .. })
    ));
    for table in [
        "sessions",
        "session_storage_generations",
        "session_history_generations",
        "session_sequences",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}

#[tokio::test]
async fn concurrent_resolve_has_one_atomic_creator_and_same_key_replay_is_stable() {
    let workspace = TestWorkspace::create("session-resolve-race").unwrap();
    let path = workspace.path().join("db");
    let left = SqliteSessionClient::open(&path).unwrap();
    let right = SqliteSessionClient::open(&path).unwrap();
    let (a, b) = tokio::join!(
        write(
            &left,
            SessionWriteOperation::Resolve {
                key: key(),
                config: config()
            }
        ),
        write(
            &right,
            SessionWriteOperation::Resolve {
                key: key(),
                config: config()
            }
        )
    );
    let a = committed(a);
    let b = committed(b);
    assert_ne!(a.created, b.created);
    assert_eq!(a.generation, b.generation);
    for receipt in [a, b] {
        let mutation = SessionWriteOperation::Resolve {
            key: receipt.operation.key.clone(),
            config: config(),
        };
        let (first, second) = tokio::join!(write(&left, mutation.clone()), write(&right, mutation));
        assert_eq!(committed(first), receipt);
        assert_eq!(committed(second), receipt);
    }
}

#[tokio::test]
async fn schema_v3_append_receipts_survive_typed_receipt_migration() {
    let workspace = TestWorkspace::create("session-receipt-migration").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    // Build real append evidence, then represent it in the published v3 columns.
    client
        .execute(SessionRequest {
            id: HostRequestId(1),
            operation: SessionOperation::Resolve { config: config() },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
    let operation = SessionWriteOperation::Append {
        key: key(),
        message: Box::new(SessionMessage {
            id: "legacy-message".into(),
            session: SessionRef { id: config().id },
            from: None,
            to: None,
            role: SessionMessageRole::User,
            created_at: "0".into(),
            payload: HostValue::String("hello".into()),
            provenance: None,
            dedup_key: None,
        }),
    };
    let expected = write(&client, operation.clone()).await;
    drop(client);
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("BEGIN IMMEDIATE;
        ALTER TABLE session_receipts RENAME TO session_receipts_v4;
        DROP INDEX session_receipts_expiry;
        CREATE TABLE session_receipts(session_id TEXT NOT NULL,operation TEXT NOT NULL,expires INTEGER NOT NULL,fingerprint TEXT NOT NULL,version TEXT NOT NULL,message_id TEXT NOT NULL,deduplicated INTEGER NOT NULL CHECK(deduplicated IN (0,1)),PRIMARY KEY(session_id,operation));
        INSERT INTO session_receipts SELECT session_id,operation,expires,fingerprint,version,message_id,deduplicated FROM session_receipts_v4;
        DROP TABLE session_receipts_v4;
        CREATE INDEX session_receipts_expiry ON session_receipts(expires);
        UPDATE etas_session_schema SET format=3;
        COMMIT;").unwrap();
    drop(db);
    let migrated = SqliteSessionClient::open(&path).unwrap();
    assert_eq!(write(&migrated, operation).await, expected);
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("SELECT format FROM etas_session_schema", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        9
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM session_messages", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
