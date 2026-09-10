use etas_host::*;

async fn execute<C: SessionClient<Error = HostError>>(
    client: &C,
    operation: SessionOperation,
) -> Result<SessionResult, HostError> {
    client
        .execute(SessionRequest {
            id: HostRequestId(1),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: Default::default(),
        })
        .await?
        .result
}
fn load(id: &str, context: ContextPolicy, cursor: Option<SessionCursor>) -> SessionOperation {
    SessionOperation::Load {
        session: SessionRef { id: id.to_owned() },
        context,
        cursor,
        limit: Some(1),
    }
}
async fn resolve<C: SessionClient<Error = HostError>>(client: &C, id: &str) {
    execute(
        client,
        SessionOperation::Resolve {
            config: SessionConfig {
                id: id.to_owned(),
                context: ContextPolicy::All,
                retention: RetentionPolicy::Forever,
            },
        },
    )
    .await
    .unwrap();
}
async fn append<C: SessionClient<Error = HostError>>(
    client: &C,
    id: &str,
    key: &str,
    payload: &str,
) {
    execute(
        client,
        SessionOperation::Append {
            message: SessionMessage {
                id: key.to_owned(),
                session: SessionRef { id: id.to_owned() },
                role: SessionMessageRole::User,
                from: None,
                to: None,
                created_at: "2026-09-09T00:00:00Z".to_owned(),
                payload: HostValue::String(payload.to_owned()),
                provenance: None,
                dedup_key: None,
            },
        },
    )
    .await
    .unwrap();
}
fn history(result: SessionResult) -> (Vec<SessionMessage>, Option<SessionCursor>) {
    let SessionResult::History {
        messages, cursor, ..
    } = result
    else {
        panic!("history result required")
    };
    (messages, cursor)
}

fn fence(result: &SessionResult) -> &etas_host::session::SessionHistoryFence {
    match result {
        SessionResult::History { fence, .. } => fence,
        _ => panic!("history required"),
    }
}

async fn fenced_pages<C: SessionClient<Error = HostError>>(client: &C) {
    resolve(client, "fenced").await;
    append(client, "fenced", "A", "secret payload").await;
    append(client, "fenced", "B", "another payload").await;
    let first = execute(client, load("fenced", ContextPolicy::All, None))
        .await
        .unwrap();
    let original = fence(&first).clone();
    let (_, cursor) = history(first);
    append(client, "fenced", "C", "later").await;
    let second = execute(client, load("fenced", ContextPolicy::All, cursor))
        .await
        .unwrap();
    assert_eq!(
        fence(&second),
        &original,
        "append cannot rewrite the selected history evidence"
    );
    let fresh = execute(client, load("fenced", ContextPolicy::All, None))
        .await
        .unwrap();
    assert_ne!(
        fence(&fresh),
        &original,
        "a new selection must include the append revision"
    );
    let claims: serde_json::Value = serde_json::from_str(original.as_token()).unwrap();
    assert_eq!(claims["claims"]["selection"]["upper"], 1);
    assert_eq!(claims["claims"]["selection"]["lower"], 0);
    assert_eq!(claims["claims"]["context_version"], 0);
    assert!(!original.as_token().contains("secret payload"));
    assert_eq!(format!("{original:?}"), "SessionHistoryFence([redacted])");
    assert_eq!(
        etas_host::session::SessionHistoryFence::from_token(
            original.as_token().to_owned(),
            &StorageLimits::default()
        )
        .unwrap(),
        original
    );
    let limits = StorageLimits {
        max_value_bytes: 64,
        ..Default::default()
    };
    assert!(
        etas_host::session::SessionHistoryFence::from_token(
            original.as_token().to_owned(),
            &limits
        )
        .is_err()
    );
    for field in [
        "format",
        "incarnation",
        "revision",
        "context_version",
        "selection",
    ] {
        let mut malformed = claims.clone();
        malformed["claims"].as_object_mut().unwrap().remove(field);
        assert!(
            etas_host::session::SessionHistoryFence::from_token(
                malformed.to_string(),
                &StorageLimits::default()
            )
            .is_err(),
            "missing {field}"
        );
    }
}

#[tokio::test]
async fn history_fence_is_bounded_and_stable_across_pages_for_both_backends() {
    fenced_pages(&InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("session-history-fence").unwrap();
    fenced_pages(&SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn sqlite_fence_secret_and_context_version_survive_reopen() {
    let workspace = TestWorkspace::create("session-history-fence-reopen").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    resolve(&client, "fenced").await;
    append(&client, "fenced", "A", "A").await;
    let first = execute(&client, load("fenced", ContextPolicy::All, None))
        .await
        .unwrap();
    let original = fence(&first).clone();
    drop(client);
    let client = SqliteSessionClient::open(&path).unwrap();
    let reopened = execute(&client, load("fenced", ContextPolicy::All, None))
        .await
        .unwrap();
    assert_eq!(fence(&reopened), &original);
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute("UPDATE sessions SET summary_text='caller context',summary_message_count=1 WHERE id='fenced'", []).unwrap();
    let changed = execute(&client, load("fenced", ContextPolicy::All, None))
        .await
        .unwrap();
    let claims: serde_json::Value = serde_json::from_str(fence(&changed).as_token()).unwrap();
    assert_eq!(claims["claims"]["context_version"], 1);
    assert_ne!(fence(&changed), &original);
    connection
        .execute("DELETE FROM session_messages WHERE session_id='fenced'", [])
        .unwrap();
    let deleted = execute(&client, load("fenced", ContextPolicy::All, None))
        .await
        .unwrap();
    assert_ne!(fence(&deleted), fence(&changed));
    let deleted: serde_json::Value = serde_json::from_str(fence(&deleted).as_token()).unwrap();
    assert_eq!(
        deleted["claims"]["selection"]["upper"], 0,
        "persistent ordinal cannot reset after deletion"
    );
    assert_eq!(
        deleted["claims"]["context_version"], 1,
        "deletion changes history, not context version"
    );
}

#[tokio::test]
async fn sqlite_missing_fence_state_is_not_recreated_on_reopen() {
    let workspace = TestWorkspace::create("session-fence-missing").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    resolve(&client, "fenced").await;
    drop(client);
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute("DELETE FROM session_fences WHERE session_id='fenced'", [])
        .unwrap();
    drop(db);
    let client = SqliteSessionClient::open(&path).unwrap();
    let error = execute(&client, load("fenced", ContextPolicy::All, None))
        .await
        .unwrap_err();
    assert_eq!(error.code, HostErrorCode::SchemaMismatch);
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM session_fences", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

async fn fixed_window<C: SessionClient<Error = HostError>>(client: &C) {
    resolve(client, "window").await;
    resolve(client, "other").await;
    append(client, "window", "A", "A").await;
    append(client, "window", "B", "B").await;
    let (first, cursor) = history(
        execute(client, load("window", ContextPolicy::LastTurns(1), None))
            .await
            .unwrap(),
    );
    assert_eq!(first[0].id, "A");
    let cursor = cursor.unwrap();
    append(client, "window", "C", "C").await;
    let (next, end) = history(
        execute(
            client,
            load("window", ContextPolicy::LastTurns(1), Some(cursor.clone())),
        )
        .await
        .unwrap(),
    );
    assert_eq!(
        next.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["B"]
    );
    assert!(
        end.is_none(),
        "new appends cannot extend an existing enumeration"
    );
    for operation in [
        load("other", ContextPolicy::LastTurns(1), Some(cursor.clone())),
        load("window", ContextPolicy::All, Some(cursor.clone())),
    ] {
        assert_eq!(
            execute(client, operation).await.unwrap_err().code,
            HostErrorCode::InvalidRequest
        );
    }
    let mut forged: serde_json::Value = serde_json::from_str(&cursor.opaque).unwrap();
    forged["cursor"]["upper"] = 2.into();
    let forged = SessionCursor {
        opaque: forged.to_string(),
    };
    assert_eq!(
        execute(
            client,
            load("window", ContextPolicy::LastTurns(1), Some(forged))
        )
        .await
        .unwrap_err()
        .code,
        HostErrorCode::InvalidRequest
    );
}

#[tokio::test]
async fn session_keyset_window_is_fixed_for_both_backends() {
    fixed_window(&InMemorySessionClient::new()).await;
    let workspace = TestWorkspace::create("session-keyset").unwrap();
    fixed_window(&SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn sqlite_history_cursor_survives_reopen_and_append_but_not_deletion() {
    let workspace = TestWorkspace::create("session-keyset-generation").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    resolve(&client, "window").await;
    for key in ["A", "B", "C"] {
        append(&client, "window", key, key).await;
    }
    let (_, cursor) = history(
        execute(&client, load("window", ContextPolicy::All, None))
            .await
            .unwrap(),
    );
    let cursor = cursor.unwrap();
    drop(client);
    let client = SqliteSessionClient::open(&path).unwrap();
    append(&client, "window", "D", "D").await;
    let (next, _) = history(
        execute(
            &client,
            load("window", ContextPolicy::All, Some(cursor.clone())),
        )
        .await
        .unwrap(),
    );
    assert_eq!(next[0].id, "B");
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("DELETE FROM session_messages WHERE message_id='B'", [])
        .unwrap();
    assert_eq!(
        execute(&client, load("window", ContextPolicy::All, Some(cursor)))
            .await
            .unwrap_err()
            .code,
        HostErrorCode::InvalidRequest
    );
}

async fn bounded_pages<C: SessionClient<Error = HostError>>(client: &C) {
    resolve(client, "large").await;
    for n in 0..30 {
        append(client, "large", &format!("m-{n}"), &"x".repeat(512)).await;
    }
    let mut cursor = None;
    let mut count = 0;
    loop {
        let (page, next) = history(
            execute(client, load("large", ContextPolicy::All, cursor))
                .await
                .unwrap(),
        );
        assert_eq!(page.len(), 1);
        count += 1;
        cursor = next;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(count, 30);
}
#[tokio::test]
async fn small_history_pages_do_not_decode_or_budget_the_entire_history() {
    let limits = StorageLimits {
        max_value_bytes: 1024,
        max_result_bytes: 2048,
        max_scan_rows: 20,
        ..Default::default()
    };
    bounded_pages(&InMemorySessionClient::with_limits(limits.clone()).unwrap()).await;
    let workspace = TestWorkspace::create("session-keyset-bounds").unwrap();
    bounded_pages(
        &SqliteSessionClient::open_with_limits(workspace.path().join("db"), limits).unwrap(),
    )
    .await;
}

#[tokio::test]
async fn sqlite_history_rejects_incomplete_summary_and_invalidates_replaced_context() {
    let workspace = TestWorkspace::create("session-keyset-summary").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    resolve(&client, "window").await;
    for key in ["A", "B"] {
        append(&client, "window", key, key).await;
    }
    let context = ContextPolicy::SummaryPlusRecent { recent: 1 };
    let (_, cursor) = history(
        execute(&client, load("window", context.clone(), None))
            .await
            .unwrap(),
    );
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute("UPDATE sessions SET summary_text='replacement', summary_message_count=2 WHERE id='window'", []).unwrap();
    assert_eq!(
        execute(&client, load("window", context.clone(), cursor))
            .await
            .unwrap_err()
            .code,
        HostErrorCode::InvalidRequest
    );
    connection
        .execute(
            "UPDATE sessions SET summary_message_count=NULL WHERE id='window'",
            [],
        )
        .unwrap();
    assert_eq!(
        execute(&client, load("window", context, None))
            .await
            .unwrap_err()
            .code,
        HostErrorCode::SchemaMismatch
    );
}
