use etas_host::session::*;
use etas_host::*;

fn authority() -> AuthorityContext {
    let mut authority = AuthorityContext::deny_all();
    authority.grants = ["read", "write"]
        .into_iter()
        .map(|action| {
            HostActionGrant::allow_with_args(
                "Memory",
                action,
                vec![ActionArgPattern::Exact(HostValue::String("s".into()))],
            )
        })
        .collect();
    authority
}
fn request(operation: SessionMaintenanceOperation) -> SessionMaintenanceRequest {
    SessionMaintenanceRequest {
        id: HostRequestId(1),
        operation,
        authority: authority(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    }
}
async fn execute<C: SessionClient<Error = HostError>>(
    c: &C,
    operation: SessionOperation,
) -> SessionResult {
    c.execute(SessionRequest {
        id: HostRequestId(1),
        operation,
        authority: authority(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    })
    .await
    .unwrap()
    .result
    .unwrap()
}
async fn setup<C: SessionClient<Error = HostError>>(c: &C) {
    execute(
        c,
        SessionOperation::Resolve {
            config: SessionConfig {
                id: "s".into(),
                context: ContextPolicy::All,
                retention: RetentionPolicy::Days(1),
            },
        },
    )
    .await;
}
fn message(id: &str, old: bool) -> SessionMessage {
    SessionMessage {
        id: id.into(),
        session: SessionRef { id: "s".into() },
        from: None,
        to: None,
        role: SessionMessageRole::User,
        created_at: if old {
            "0".into()
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string()
        },
        payload: HostValue::String(format!("sensitive payload {id}")),
        provenance: Some(HostValue::String("original provenance".into())),
        dedup_key: Some(id.into()),
    }
}
async fn append<C: SessionClient<Error = HostError>>(c: &C, id: &str, old: bool) {
    execute(
        c,
        SessionOperation::Append {
            message: message(id, old),
        },
    )
    .await;
}
async fn history<C: SessionClient<Error = HostError>>(c: &C) -> SessionResult {
    execute(
        c,
        SessionOperation::Load {
            session: SessionRef { id: "s".into() },
            context: ContextPolicy::All,
            cursor: None,
            limit: Some(1),
        },
    )
    .await
}
fn intent(
    fence: SessionHistoryFence,
    after: i64,
    limit: u32,
    key: StorageOperationKey,
) -> SessionRetentionIntent {
    SessionRetentionIntent::prepare(
        SessionRef { id: "s".into() },
        fence,
        after,
        limit,
        key,
        &StorageLimits::default(),
    )
    .unwrap()
}
fn key() -> StorageOperationKey {
    StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap()
}
async fn maintain<C: SessionMaintenanceClient>(
    c: &C,
    intent: SessionRetentionIntent,
) -> SessionRetentionOutcome {
    let SessionMaintenanceResult::Outcome(outcome) = c
        .maintain(request(SessionMaintenanceOperation::Retain(Box::new(
            intent,
        ))))
        .await
        .unwrap()
        .result
        .unwrap()
    else {
        panic!("outcome")
    };
    outcome
}
fn receipt(outcome: SessionRetentionOutcome) -> SessionRetentionReceipt {
    match outcome {
        WriteOutcome::Committed(r) => {
            assert!(r.progress.deleted_messages > 0);
            r
        }
        WriteOutcome::NotCommitted {
            reason: SessionRetentionRejection::NoChange(r),
            ..
        } => {
            assert_eq!(r.progress.deleted_messages, 0);
            r
        }
        other => panic!("{other:?}"),
    }
}

async fn bounded_sweep<C: SessionClient<Error = HostError> + SessionMaintenanceClient>(c: &C) {
    setup(c).await;
    for n in 0..101 {
        append(c, &format!("old-{n}"), true).await;
    }
    append(c, "recent-1", false).await;
    append(c, "recent-2", false).await;
    let SessionResult::History {
        fence,
        cursor: Some(cursor),
        ..
    } = history(c).await
    else {
        panic!("page")
    };
    let publication = SessionContextPublication::prepare(
        SessionRef { id: "s".into() },
        fence,
        SessionContextContent {
            text: "caller context".into(),
            provenance: [("producer".into(), "application".into())].into(),
        },
        &StorageLimits::default(),
    )
    .unwrap();
    c.write(SessionWriteRequest {
        id: HostRequestId(2),
        operation: SessionWriteOperation::PublishContext(Box::new(publication.clone())),
        authority: authority(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    })
    .await
    .unwrap()
    .result
    .unwrap();
    let SessionResult::History {
        fence,
        cursor: Some(after_publication),
        ..
    } = history(c).await
    else {
        panic!("page")
    };
    let first = intent(fence.clone(), -1, 17, key());
    let first_receipt = receipt(maintain(c, first.clone()).await);
    assert_eq!(first_receipt.progress.deleted_messages, 17);
    append(c, "late-old", true).await;
    let mut total = first_receipt.progress.deleted_messages;
    let mut removed_dedup = first_receipt.progress.deleted_dedup_keys;
    let mut next = first_receipt.progress.next_after;
    while let Some(after) = next {
        let r = receipt(maintain(c, intent(fence.clone(), after, 17, key())).await);
        assert!(r.progress.scanned <= 17);
        total += r.progress.deleted_messages;
        removed_dedup += r.progress.deleted_dedup_keys;
        next = r.progress.next_after;
    }
    assert_eq!((total, removed_dedup), (101, 101));
    assert_eq!(receipt(maintain(c, first.clone()).await), first_receipt);
    let lookup = c
        .maintain(request(SessionMaintenanceOperation::Reconcile {
            session: SessionRef { id: "s".into() },
            operation: first.operation.clone(),
        }))
        .await
        .unwrap()
        .result
        .unwrap();
    assert_eq!(
        lookup,
        SessionMaintenanceResult::Receipt(ReceiptLookup::Found(first_receipt))
    );
    for cursor in [cursor, after_publication] {
        assert!(
            c.execute(SessionRequest {
                id: HostRequestId(4),
                operation: SessionOperation::Load {
                    session: SessionRef { id: "s".into() },
                    context: ContextPolicy::All,
                    cursor: Some(cursor),
                    limit: Some(1)
                },
                authority: authority(),
                trace: TraceContext::root(TraceId(1)),
                budget: Default::default()
            })
            .await
            .unwrap()
            .result
            .is_err()
        );
    }
    let SessionResult::History {
        fence: fresh,
        published_context: Some(context),
        messages,
        ..
    } = history(c).await
    else {
        panic!("context lost")
    };
    assert_eq!(context.content, publication.content);
    assert_eq!(context.fence, publication.fence);
    assert_eq!(messages[0].id, "recent-1");
    // The old sweep cannot consume an append beyond its fixed upper bound.
    let r = receipt(maintain(c, intent(fresh, -1, 17, key())).await);
    assert_eq!(r.progress.deleted_messages, 1);
    // New appends receive ordinals above all previous messages, including deleted ones.
    let msg = message("after-retention", false);
    let result = c
        .write(SessionWriteRequest {
            id: HostRequestId(5),
            operation: SessionWriteOperation::Append {
                key: key(),
                message: Box::new(msg),
            },
            authority: authority(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
    let SessionWriteResult::Outcome(WriteOutcome::Committed(SessionWriteReceipt::Append(r))) =
        result
    else {
        panic!("append")
    };
    let ordinal =
        u64::from_str_radix(r.version.as_token().rsplit(':').next().unwrap(), 16).unwrap();
    assert_eq!(ordinal, 104);
}

#[tokio::test]
async fn retention_is_bounded_preserves_context_and_keeps_monotonic_ordinals() {
    bounded_sweep(&InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("retention-sweep").unwrap();
    let path = workspace.path().join("db");
    bounded_sweep(&SqliteSessionClient::open(&path).unwrap()).await;
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM session_messages", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM session_contexts", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

async fn validation<C: SessionClient<Error = HostError> + SessionMaintenanceClient>(c: &C) {
    setup(c).await;
    append(c, "old", true).await;
    let SessionResult::History { fence, .. } = history(c).await else {
        panic!("history")
    };
    let original = intent(fence.clone(), -1, 1, key());
    let mut denied = request(SessionMaintenanceOperation::Retain(Box::new(
        original.clone(),
    )));
    denied.authority = AuthorityContext::deny_all();
    assert_eq!(
        c.maintain(denied).await.unwrap_err().code,
        HostErrorCode::AuthorityDenied
    );
    let mut changed = original.clone();
    changed.scan_limit = 2;
    assert_eq!(
        c.maintain(request(SessionMaintenanceOperation::Retain(Box::new(
            changed
        ))))
        .await
        .unwrap_err()
        .code,
        HostErrorCode::InvalidRequest
    );
    let r = receipt(maintain(c, original.clone()).await);
    assert_eq!(r.progress.deleted_messages, 1);
    let other = intent(fence, 0, 1, original.operation.key.clone());
    assert!(matches!(
        maintain(c, other).await,
        WriteOutcome::NotCommitted {
            reason: SessionRetentionRejection::Rejected(_),
            ..
        }
    ));
    let mut denied = request(SessionMaintenanceOperation::Reconcile {
        session: original.session,
        operation: original.operation,
    });
    denied.authority = AuthorityContext::deny_all();
    assert_eq!(
        c.maintain(denied).await.unwrap_err().code,
        HostErrorCode::AuthorityDenied
    );
}

#[tokio::test]
async fn retention_and_reconciliation_require_current_authority_and_bound_intent() {
    validation(&InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("retention-validation").unwrap();
    validation(&SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn sqlite_retention_receipts_survive_reopen_and_share_global_capacity() {
    let workspace = TestWorkspace::create("retention-reopen").unwrap();
    let path = workspace.path().join("db");
    let limits = StorageLimits {
        max_receipts: 1,
        ..Default::default()
    };
    let c = SqliteSessionClient::open_with_limits(&path, limits.clone()).unwrap();
    setup(&c).await;
    append(&c, "old", true).await;
    let SessionResult::History { fence, .. } = history(&c).await else {
        panic!("history")
    };
    let original = intent(fence, -1, 1, key());
    let expected = receipt(maintain(&c, original.clone()).await);
    drop(c);
    let c = SqliteSessionClient::open_with_limits(&path, limits).unwrap();
    assert_eq!(receipt(maintain(&c, original).await), expected);
    let SessionResult::History { fence, .. } = history(&c).await else {
        panic!("history")
    };
    assert!(matches!(
        maintain(&c, intent(fence, -1, 1, key())).await,
        WriteOutcome::NotCommitted {
            reason: SessionRetentionRejection::Rejected(HostError {
                code: HostErrorCode::BudgetExceeded,
                ..
            }),
            ..
        }
    ));
    let result = c
        .write(SessionWriteRequest {
            id: HostRequestId(5),
            operation: SessionWriteOperation::Append {
                key: key(),
                message: Box::new(message("new", false)),
            },
            authority: authority(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await
        .unwrap()
        .result
        .unwrap();
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
}

async fn append_replay<C: SessionClient<Error = HostError> + SessionMaintenanceClient>(c: &C) {
    setup(c).await;
    let original = SessionWriteRequest {
        id: HostRequestId(5),
        operation: SessionWriteOperation::Append {
            key: key(),
            message: Box::new(message("old", true)),
        },
        authority: authority(),
        trace: TraceContext::root(TraceId(1)),
        budget: Default::default(),
    };
    let expected = c.write(original.clone()).await.unwrap().result.unwrap();
    let SessionResult::History { fence, .. } = history(c).await else {
        panic!("history")
    };
    let SessionWriteOperation::Append {
        key: append_key, ..
    } = &original.operation
    else {
        unreachable!()
    };
    assert!(matches!(
        maintain(c, intent(fence.clone(), -1, 1, append_key.clone())).await,
        WriteOutcome::NotCommitted {
            reason: SessionRetentionRejection::Rejected(HostError {
                code: HostErrorCode::InvalidRequest,
                ..
            }),
            ..
        }
    ));
    let r = receipt(maintain(c, intent(fence.clone(), -1, 1, key())).await);
    assert_eq!(
        (r.progress.deleted_messages, r.progress.deleted_dedup_keys),
        (1, 1)
    );
    assert_eq!(
        c.write(original.clone()).await.unwrap().result.unwrap(),
        expected
    );
    assert!(matches!(
        maintain(c, intent(fence, -1, 1, key())).await,
        WriteOutcome::NotCommitted {
            reason: SessionRetentionRejection::NoChange(_),
            ..
        }
    ));
    // Dedup is explicitly scoped to retained messages. An original operation
    // still replays its receipt, but a new operation no longer reuses deleted payload.
    let mut new = original;
    let SessionWriteOperation::Append {
        key: ref mut identity,
        ..
    } = new.operation
    else {
        unreachable!()
    };
    *identity = key();
    let SessionWriteResult::Outcome(WriteOutcome::Committed(SessionWriteReceipt::Append(r))) =
        c.write(new).await.unwrap().result.unwrap()
    else {
        panic!("append")
    };
    assert!(!r.deduplicated);
    assert!(r.version.as_token().ends_with(":0000000000000001"));
}

async fn scoped<C, F, Fut>(client: &C, cancelled: bool, dispatch: F)
where
    C: SessionClient<Error = HostError> + SessionMaintenanceClient,
    F: FnOnce(SessionMaintenanceRequest, etas_host::execution::OperationContext) -> Fut,
    Fut: std::future::Future<Output = Result<SessionMaintenanceResponse, HostError>>,
{
    use etas_host::execution::{
        CancellationReason, ExecutionScope, ExternalOutcome, OperationResponse,
    };
    setup(client).await;
    append(client, "expired", true).await;
    let SessionResult::History { fence, .. } = history(client).await else {
        panic!("history")
    };
    let intent = intent(fence, -1, 1, key());
    let operation = intent.operation.clone();
    let scope = ExecutionScope::new();
    let registration = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    registration.begin_dispatch().unwrap();
    if cancelled {
        scope
            .cancel_source()
            .stop(CancellationReason::Requested)
            .unwrap();
    }
    let response = dispatch(
        request(SessionMaintenanceOperation::Retain(Box::new(
            intent.clone(),
        ))),
        registration.context().clone(),
    )
    .await
    .unwrap();
    let outcome = response.external_outcome();
    if cancelled {
        assert_eq!(
            outcome,
            ExternalOutcome::StorageWrite(StorageWriteEvidence {
                operation: operation.clone(),
                status: CommitStatus::NotCommitted,
            })
        );
        assert!(matches!(
            &response.result,
            Ok(SessionMaintenanceResult::Outcome(
                WriteOutcome::NotCommitted {
                    reason: SessionRetentionRejection::Rejected(HostError {
                        code: HostErrorCode::Cancelled,
                        ..
                    }),
                    ..
                }
            ))
        ));
    } else {
        assert!(
            matches!(&outcome,ExternalOutcome::StorageWrite(StorageWriteEvidence {operation:actual,status:CommitStatus::Committed{..}}) if actual==&operation)
        );
    }
    registration.complete(outcome.clone(), vec![]).unwrap();
    scope.finish_body(false).unwrap();
    let report = scope.join().await.unwrap();
    assert_eq!(
        report.operations().len(),
        if cancelled { 1 } else { 2 },
        "dispatch and owned storage operation"
    );
    assert!(
        report
            .operations()
            .iter()
            .all(|op| op.outcome() == Some(&outcome))
    );
    if cancelled {
        // The original fence and operation remain usable because no deletion
        // or receipt publication occurred before dispatch.
        let committed = receipt(maintain(client, intent).await);
        assert_eq!(committed.progress.deleted_messages, 1);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn retention_scoped_execution_preserves_commit_evidence_until_cleanup_settles() {
    let memory = InMemorySessionClient::new();
    let dispatch = memory.clone();
    scoped(&memory, false, move |request, context| async move {
        dispatch.maintain_scoped(request, &context).await
    })
    .await;
    let workspace = TestWorkspace::create("retention-owned-operation").unwrap();
    let sqlite = SqliteSessionClient::open(workspace.path().join("db")).unwrap();
    let dispatch = sqlite.clone();
    scoped(&sqlite, false, move |request, context| async move {
        dispatch.maintain_scoped(request, &context).await
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn retention_cancelled_before_dispatch_reports_non_commit_without_deleting() {
    let memory = InMemorySessionClient::new();
    let dispatch = memory.clone();
    scoped(&memory, true, move |request, context| async move {
        dispatch.maintain_scoped(request, &context).await
    })
    .await;
    let workspace = TestWorkspace::create("retention-cancel-before-dispatch").unwrap();
    let sqlite = SqliteSessionClient::open(workspace.path().join("db")).unwrap();
    let dispatch = sqlite.clone();
    scoped(&sqlite, true, move |request, context| async move {
        dispatch.maintain_scoped(request, &context).await
    })
    .await;
}

#[tokio::test]
async fn retention_reports_dedup_loss_without_removing_append_commit_evidence() {
    append_replay(&InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("retention-append-evidence").unwrap();
    append_replay(&SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn retention_trace_is_redacted_and_fingerprints_the_complete_selection() {
    let client = InMemorySessionClient::new();
    setup(&client).await;
    append(&client, "private-message", true).await;
    let SessionResult::History { fence, .. } = history(&client).await else {
        panic!("history")
    };
    let first = intent(fence.clone(), -1, 1, key());
    let second = intent(fence, 0, 1, key());
    let digest_key = HostTraceDigestKey::generate().unwrap();
    let a = HostTraceMetadata::from_payload(
        &request(SessionMaintenanceOperation::Retain(Box::new(first.clone()))).trace_payload(),
        &digest_key,
    )
    .unwrap();
    let b = HostTraceMetadata::from_payload(
        &request(SessionMaintenanceOperation::Retain(Box::new(second))).trace_payload(),
        &digest_key,
    )
    .unwrap();
    assert!(a.fields.iter().all(|f| f.value.is_none()));
    assert!(!format!("{a:?}").contains(first.fence.as_token()));
    assert_ne!(a.payload_digest, b.payload_digest);
}
