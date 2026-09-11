use etas_host::session::*;
use etas_host::*;

#[test]
fn publication_canonical_validation_runs_inside_admitted_worker() {
    use std::future::Future;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let client = InMemorySessionClient::default();
        setup(&client).await;
        let mut publication = prepare(&client, "caller supplied context").await;
        publication
            .content
            .text
            .push_str(" tampered after preparation");
        let (started, ready) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let busy = tokio::task::spawn_blocking(move || {
            started.send(()).unwrap();
            wait.recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        });
        ready.await.unwrap();
        let mut pending = Box::pin(write(
            &client,
            SessionWriteOperation::PublishContext(Box::new(publication)),
        ));
        std::future::poll_fn(|cx| {
            assert!(
                pending.as_mut().poll(cx).is_pending(),
                "canonical encoding happened before worker admission"
            );
            std::task::Poll::Ready(())
        })
        .await;
        release.send(()).unwrap();
        busy.await.unwrap();
        let error = pending.await.unwrap_err();
        assert_eq!(error.code, HostErrorCode::InvalidRequest);
        assert!(error.message.contains("different request"));
        let SessionResult::History { summary, .. } = load(&client).await else {
            panic!("history")
        };
        assert!(
            summary.is_none(),
            "invalid publication changed stored context"
        );
    });
}

async fn execute<C: SessionClient<Error = HostError>>(
    client: &C,
    operation: SessionOperation,
) -> SessionResult {
    client
        .execute(SessionRequest {
            id: HostRequestId(1),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap()
}
async fn write<C: SessionClient<Error = HostError>>(
    client: &C,
    operation: SessionWriteOperation,
) -> Result<SessionWriteResult, HostError> {
    client
        .write(SessionWriteRequest {
            id: HostRequestId(2),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await?
        .result
}
async fn setup<C: SessionClient<Error = HostError>>(client: &C) {
    execute(
        client,
        SessionOperation::Resolve {
            config: SessionConfig {
                id: "session".into(),
                context: ContextPolicy::All,
                retention: RetentionPolicy::Forever,
            },
        },
    )
    .await;
    append(client, "first").await;
}
async fn append<C: SessionClient<Error = HostError>>(client: &C, id: &str) {
    execute(
        client,
        SessionOperation::Append {
            message: SessionMessage {
                id: id.into(),
                session: SessionRef {
                    id: "session".into(),
                },
                from: None,
                to: None,
                role: SessionMessageRole::User,
                created_at: "0".into(),
                payload: HostValue::String(id.into()),
                provenance: None,
                dedup_key: None,
            },
        },
    )
    .await;
}
async fn load<C: SessionClient<Error = HostError>>(client: &C) -> SessionResult {
    execute(
        client,
        SessionOperation::Load {
            session: SessionRef {
                id: "session".into(),
            },
            context: ContextPolicy::All,
            cursor: None,
            limit: Some(100),
        },
    )
    .await
}
fn content(text: &str) -> SessionContextContent {
    SessionContextContent {
        text: text.into(),
        provenance: [
            ("producer".into(), "application-agent".into()),
            ("tokenizer".into(), "caller-owned".into()),
        ]
        .into(),
    }
}
async fn prepare<C: SessionClient<Error = HostError>>(
    client: &C,
    text: &str,
) -> SessionContextPublication {
    let SessionResult::History { session, fence, .. } = load(client).await else {
        panic!("history")
    };
    SessionContextPublication::prepare(session, fence, content(text), &StorageLimits::default())
        .unwrap()
}
async fn publish<C: SessionClient<Error = HostError>>(
    client: &C,
    publication: SessionContextPublication,
) -> SessionContextOutcome {
    match write(
        client,
        SessionWriteOperation::PublishContext(Box::new(publication)),
    )
    .await
    .unwrap()
    {
        SessionWriteResult::Context(outcome) => outcome,
        other => panic!("{other:?}"),
    }
}
fn committed(outcome: SessionContextOutcome) -> SessionContextReceipt {
    match outcome {
        WriteOutcome::Committed(r) => r,
        other => panic!("{other:?}"),
    }
}

async fn replay_and_conflict<C: SessionClient<Error = HostError>>(client: &C) {
    setup(client).await;
    let first = prepare(client, "caller-produced context").await;
    let stale = prepare(client, "stale content").await;
    let receipt = committed(publish(client, first.clone()).await);
    assert_eq!(receipt.context_version, 1);
    let SessionResult::History {
        published_context: Some(context),
        messages,
        ..
    } = load(client).await
    else {
        panic!("published context")
    };
    assert_eq!(context.content, first.content);
    assert_eq!(context.fence, first.fence);
    assert_eq!(messages.len(), 1, "publication must not delete history");
    let rejected = publish(client, stale.clone()).await;
    assert_eq!(
        rejected,
        WriteOutcome::NotCommitted {
            operation: stale.operation.clone(),
            reason: SessionContextRejection::StaleHistory
        }
    );
    append(client, "later").await;
    let newer = prepare(client, "new context").await;
    assert_eq!(committed(publish(client, newer).await).context_version, 2);
    assert_eq!(
        committed(publish(client, first.clone()).await),
        receipt,
        "receipt replay precedes current fence checks"
    );
    assert_eq!(
        publish(client, stale.clone()).await,
        rejected,
        "a confirmed stale rejection is retained too"
    );
    assert_eq!(
        write(
            client,
            SessionWriteOperation::ReconcileContext {
                session: stale.session.clone(),
                operation: stale.operation.clone()
            }
        )
        .await
        .unwrap(),
        SessionWriteResult::ContextReceipt(ReceiptLookup::Found(
            SessionContextEvidence::NotCommitted {
                operation: stale.operation,
                reason: SessionContextRejection::StaleHistory
            }
        ))
    );
    let mut changed = first.clone();
    changed
        .content
        .provenance
        .insert("producer".into(), "different producer".into());
    changed.operation = context_operation_ref(
        &changed.session,
        &changed.fence,
        &changed.content,
        first.operation.key.clone(),
        &StorageLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        publish(client, changed).await,
        WriteOutcome::NotCommitted {
            reason: SessionContextRejection::Rejected(HostError {
                code: HostErrorCode::InvalidRequest,
                ..
            }),
            ..
        }
    ));
    let SessionResult::History {
        published_context: Some(context),
        messages,
        ..
    } = load(client).await
    else {
        panic!("context")
    };
    assert_eq!(context.content.text, "new context");
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn both_backends_conditionally_publish_caller_content_and_replay_confirmed_outcomes() {
    replay_and_conflict(&InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("session-context-replay").unwrap();
    replay_and_conflict(&SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn sqlite_context_and_receipt_survive_reopen_with_full_provenance() {
    let workspace = TestWorkspace::create("session-context-reopen").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    setup(&client).await;
    let publication = prepare(&client, "persisted application content").await;
    let receipt = committed(publish(&client, publication.clone()).await);
    drop(client);
    let client = SqliteSessionClient::open(&path).unwrap();
    append(&client, "after-reopen").await;
    assert_eq!(
        committed(publish(&client, publication.clone()).await),
        receipt
    );
    assert_eq!(
        write(
            &client,
            SessionWriteOperation::ReconcileContext {
                session: publication.session.clone(),
                operation: publication.operation.clone()
            }
        )
        .await
        .unwrap(),
        SessionWriteResult::ContextReceipt(ReceiptLookup::Found(
            SessionContextEvidence::Committed(receipt)
        ))
    );
    let SessionResult::History {
        published_context: Some(context),
        ..
    } = load(&client).await
    else {
        panic!("context")
    };
    assert_eq!(context.content, publication.content);
    assert_eq!(context.fence, publication.fence);
}

async fn invalid_fence<C: SessionClient<Error = HostError>>(client: &C) {
    setup(client).await;
    let original = prepare(client, "context").await;
    for field in ["revision", "context_version"] {
        let mut token: serde_json::Value = serde_json::from_str(original.fence.as_token()).unwrap();
        token["claims"][field] = if field == "revision" {
            "0".repeat(32).into()
        } else {
            99.into()
        };
        let forged =
            SessionHistoryFence::from_token(token.to_string(), &StorageLimits::default()).unwrap();
        let request = SessionContextPublication::prepare(
            original.session.clone(),
            forged,
            content("forged"),
            &StorageLimits::default(),
        )
        .unwrap();
        let operation = request.operation.clone();
        assert!(matches!(
            publish(client, request).await,
            WriteOutcome::NotCommitted {
                reason: SessionContextRejection::Rejected(_),
                ..
            }
        ));
        assert_eq!(
            write(
                client,
                SessionWriteOperation::ReconcileContext {
                    session: original.session.clone(),
                    operation
                }
            )
            .await
            .unwrap(),
            SessionWriteResult::ContextReceipt(ReceiptLookup::Unresolved)
        );
    }
    append(client, "advanced").await;
    assert!(matches!(
        publish(client, original).await,
        WriteOutcome::NotCommitted {
            reason: SessionContextRejection::StaleHistory,
            ..
        }
    ));
    assert!(matches!(
        load(client).await,
        SessionResult::History {
            published_context: None,
            ..
        }
    ));
}

#[tokio::test]
async fn both_backends_reject_forged_fences_and_history_advanced_by_append() {
    invalid_fence(&InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("session-context-fence").unwrap();
    invalid_fence(&SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn two_sqlite_publishers_cannot_both_commit_against_one_fence() {
    let workspace = TestWorkspace::create("session-context-race").unwrap();
    let path = workspace.path().join("db");
    let a = SqliteSessionClient::open(&path).unwrap();
    let b = SqliteSessionClient::open(&path).unwrap();
    setup(&a).await;
    let one = prepare(&a, "one").await;
    let two = prepare(&b, "two").await;
    let (one, two) = tokio::join!(publish(&a, one), publish(&b, two));
    let outcomes = [one, two];
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(o, WriteOutcome::Committed(_)))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| matches!(
                o,
                WriteOutcome::NotCommitted {
                    reason: SessionContextRejection::StaleHistory,
                    ..
                }
            ))
            .count(),
        1
    );
}

async fn shared_receipt_capacity<C: SessionClient<Error = HostError>>(client: &C) {
    setup(client).await;
    let first = prepare(client, "retained context").await;
    let receipt = committed(publish(client, first.clone()).await);
    let second = prepare(client, "must not replace").await;
    assert!(matches!(
        publish(client, second).await,
        WriteOutcome::NotCommitted {
            reason: SessionContextRejection::Rejected(HostError {
                code: HostErrorCode::BudgetExceeded,
                ..
            }),
            ..
        }
    ));
    let config = SessionConfig {
        id: first.session.id.clone(),
        context: ContextPolicy::All,
        retention: RetentionPolicy::Forever,
    };
    let reused_key = write(
        client,
        SessionWriteOperation::Resolve {
            key: first.operation.key.clone(),
            config: config.clone(),
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        reused_key,
        SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: HostError {
                code: HostErrorCode::InvalidRequest,
                ..
            },
            ..
        })
    ));
    let new_key = write(
        client,
        SessionWriteOperation::Resolve {
            key: StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap(),
            config,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        new_key,
        SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
            reason: HostError {
                code: HostErrorCode::BudgetExceeded,
                ..
            },
            ..
        })
    ));
    assert_eq!(
        committed(publish(client, first).await),
        receipt,
        "replay remains available at capacity"
    );
    let SessionResult::History {
        published_context: Some(context),
        ..
    } = load(client).await
    else {
        panic!("context")
    };
    assert_eq!(context.content.text, "retained context");
    assert_eq!(context.version, 1);
}

#[tokio::test]
async fn context_receipts_share_capacity_and_identity_namespace_with_other_mutations() {
    let limits = StorageLimits {
        max_receipts: 1,
        ..Default::default()
    };
    shared_receipt_capacity(&InMemorySessionClient::with_limits(limits.clone()).unwrap()).await;
    let workspace = TestWorkspace::create("context-receipt-capacity").unwrap();
    shared_receipt_capacity(
        &SqliteSessionClient::open_with_limits(workspace.path().join("db"), limits).unwrap(),
    )
    .await;
}

#[tokio::test]
async fn sqlite_context_decode_obeys_current_configured_limits_not_defaults() {
    let workspace = TestWorkspace::create("context-decode-limits").unwrap();
    let path = workspace.path().join("db");
    let wide = StorageLimits {
        max_value_bytes: 3 * 1024 * 1024,
        ..Default::default()
    };
    let client = SqliteSessionClient::open_with_limits(&path, wide.clone()).unwrap();
    setup(&client).await;
    let SessionResult::History { session, fence, .. } = load(&client).await else {
        panic!("history")
    };
    let publication = SessionContextPublication::prepare(
        session,
        fence,
        content(&"x".repeat(StorageLimits::default().max_value_bytes + 100)),
        &wide,
    )
    .unwrap();
    committed(publish(&client, publication.clone()).await);
    let SessionResult::History {
        published_context: Some(context),
        ..
    } = load(&client).await
    else {
        panic!("context")
    };
    assert_eq!(context.content, publication.content);
    drop(client);
    let strict = SqliteSessionClient::open(&path).unwrap();
    let result = strict
        .execute(SessionRequest {
            id: HostRequestId(3),
            operation: SessionOperation::Load {
                session: publication.session.clone(),
                context: ContextPolicy::All,
                cursor: None,
                limit: Some(1),
            },
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result;
    assert!(matches!(
        result,
        Err(HostError {
            code: HostErrorCode::BudgetExceeded,
            ..
        })
    ));
    drop(strict);
    let client = SqliteSessionClient::open_with_limits(path, wide).unwrap();
    assert!(matches!(
        load(&client).await,
        SessionResult::History {
            published_context: Some(_),
            ..
        }
    ));
}

#[tokio::test]
async fn publication_trace_redacts_content_but_binds_provenance_in_digest() {
    let client = InMemorySessionClient::new();
    setup(&client).await;
    let mut publication = prepare(&client, "private application context").await;
    let key = HostTraceDigestKey::generate().unwrap();
    let original = HostTraceMetadata::from_payload(&publication.trace_payload(), &key).unwrap();
    assert!(original.fields.iter().all(|field| field.value.is_none()));
    let encoded = format!("{original:?}");
    assert!(!encoded.contains("private application context"));
    assert!(!encoded.contains(publication.fence.as_token()));
    publication
        .content
        .provenance
        .insert("producer".into(), "another caller".into());
    let changed = HostTraceMetadata::from_payload(&publication.trace_payload(), &key).unwrap();
    assert_ne!(original.payload_digest, changed.payload_digest);
}

#[tokio::test]
async fn sqlite_legacy_context_replacement_invalidates_publication_once() {
    let workspace = TestWorkspace::create("context-replacement-version").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    setup(&client).await;
    let initial = prepare(&client, "published context").await;
    let receipt = committed(publish(&client, initial.clone()).await);
    assert_eq!(receipt.context_version, 1);
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE sessions SET summary_text='legacy replacement',summary_message_count=1 WHERE id='session'", []).unwrap();
    let version: i64 = db
        .query_row(
            "SELECT context_version FROM session_fences WHERE session_id='session'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version, 2, "replacement must not have two version owners");
    assert!(matches!(
        load(&client).await,
        SessionResult::History {
            published_context: None,
            ..
        }
    ));
    let next = prepare(&client, "next publication").await;
    assert_eq!(committed(publish(&client, next).await).context_version, 3);
    assert_eq!(committed(publish(&client, initial).await), receipt);
}

#[tokio::test]
async fn disabled_compaction_migration_preserves_published_context_and_receipt() {
    let workspace = TestWorkspace::create("session-disabled-compaction-migration").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    setup(&client).await;
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE sessions SET config_json=json_set(config_json,'$.compaction',json('{\"kind\":\"None\"}'))", []).unwrap();
    let publication = prepare(&client, "application context retained by migration").await;
    let receipt = committed(publish(&client, publication.clone()).await);
    db.execute("UPDATE etas_session_schema SET format=7", [])
        .unwrap();
    drop(client);
    let client = SqliteSessionClient::open(&path).unwrap();
    let SessionResult::History {
        published_context: Some(context),
        messages,
        ..
    } = load(&client).await
    else {
        panic!("context lost during migration")
    };
    assert_eq!(context.content, publication.content);
    assert_eq!(context.fence, publication.fence);
    assert_eq!(context.version, receipt.context_version);
    assert_eq!(messages.len(), 1);
    assert_eq!(committed(publish(&client, publication).await), receipt);
    let config: String = db
        .query_row("SELECT config_json FROM sessions", [], |row| row.get(0))
        .unwrap();
    assert!(
        serde_json::from_str::<serde_json::Value>(&config)
            .unwrap()
            .get("compaction")
            .is_none()
    );
    let version: i64 = db
        .query_row("SELECT format FROM etas_session_schema", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 9);
}

#[tokio::test]
async fn enabled_compaction_migration_fails_without_changing_stored_state() {
    let workspace = TestWorkspace::create("session-enabled-compaction-migration").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    setup(&client).await;
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("UPDATE sessions SET config_json=json_set(config_json,'$.compaction',json('{\"kind\":\"SummarizeWhen\",\"limit\":1}'))", []).unwrap();
    let publication = prepare(&client, "must survive rejected migration").await;
    committed(publish(&client, publication).await);
    db.execute("UPDATE etas_session_schema SET format=7", [])
        .unwrap();
    let stored = || {
        db.query_row("SELECT config_json,content,context_version FROM sessions JOIN session_contexts ON sessions.id=session_contexts.session_id JOIN session_fences ON sessions.id=session_fences.session_id", [], |row| Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,i64>(2)?))).unwrap()
    };
    let before = stored();
    drop(client);
    let error = match SqliteSessionClient::open(&path) {
        Ok(_) => panic!("enabled obsolete policy accepted"),
        Err(error) => error,
    };
    assert_eq!(error.code, HostErrorCode::SchemaMismatch);
    assert!(
        error
            .message
            .contains("history_page/prepare_context/publish_context")
    );
    assert_eq!(stored(), before);
    let version: i64 = db
        .query_row("SELECT format FROM etas_session_schema", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 7, "failed migration must roll back format changes");
}

#[tokio::test]
async fn malformed_compaction_options_have_explicit_migration_errors() {
    for value in [
        "null",
        "42",
        "\"None\"",
        "[]",
        "{}",
        "{\"kind\":\"None\",\"extra\":1}",
    ] {
        let workspace = TestWorkspace::create("session-malformed-compaction-migration").unwrap();
        let path = workspace.path().join("db");
        let client = SqliteSessionClient::open(&path).unwrap();
        setup(&client).await;
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute(
            "UPDATE sessions SET config_json=json_set(config_json,'$.compaction',json(?1))",
            [value],
        )
        .unwrap();
        drop(client);
        let error = match SqliteSessionClient::open(&path) {
            Ok(_) => panic!("accepted malformed option: {value}"),
            Err(error) => error,
        };
        assert_eq!(
            error.code,
            HostErrorCode::SchemaMismatch,
            "{value}: {error:?}"
        );
        assert!(
            error
                .message
                .contains("SessionConfig.compaction is obsolete")
        );
    }
}
