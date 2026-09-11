use etas_host::{
    AuthorityContext, ContextPolicy, HostRequestId, HostValue, InMemorySessionClient,
    RetentionPolicy, SessionClient, SessionConfig, SessionMessage, SessionMessageRole,
    SessionOperation, SessionRef, SessionRequest, SessionResult, SqliteSessionClient, TraceContext,
    TraceId,
};
use std::path::PathBuf;

#[tokio::test]
async fn in_memory_session_resolves_and_loads_last_turns() {
    let client = InMemorySessionClient::new();
    resolve(&client, config("case-1")).await;
    for index in 0..6 {
        append(
            &client,
            message(
                "case-1",
                &format!("msg-{index}"),
                &format!("payload-{index}"),
                None,
            ),
        )
        .await;
    }

    let result = execute(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "case-1".to_owned(),
            },
            context: ContextPolicy::LastTurns(2),
            cursor: None,
            limit: None,
        },
    )
    .await;
    let SessionResult::History { messages, .. } = result else {
        panic!("expected history result");
    };
    assert_eq!(
        messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<Vec<_>>(),
        vec!["msg-2", "msg-3", "msg-4", "msg-5"]
    );
}

#[tokio::test]
async fn in_memory_session_append_is_deduplicated_by_key() {
    let client = InMemorySessionClient::new();
    resolve(&client, config("case-2")).await;
    let first = append(
        &client,
        message("case-2", "msg-a", "first", Some("turn:1".to_owned())),
    )
    .await;
    assert!(matches!(
        first,
        SessionResult::Appended {
            deduplicated: false,
            ..
        }
    ));

    let second = append(
        &client,
        message("case-2", "msg-b", "first", Some("turn:1".to_owned())),
    )
    .await;
    let SessionResult::Appended {
        message,
        deduplicated,
    } = second
    else {
        panic!("expected append result");
    };
    assert!(deduplicated);
    assert_eq!(message.id, "msg-a");
    assert_eq!(message.payload, HostValue::String("first".to_owned()));
}

#[tokio::test]
async fn in_memory_session_load_supports_cursor_and_limit() {
    let client = InMemorySessionClient::new();
    resolve(&client, config("case-3")).await;
    for index in 0..5 {
        append(
            &client,
            message(
                "case-3",
                &format!("msg-{index}"),
                &format!("payload-{index}"),
                None,
            ),
        )
        .await;
    }

    let first = execute(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "case-3".to_owned(),
            },
            context: ContextPolicy::All,
            cursor: None,
            limit: Some(1),
        },
    )
    .await;
    let SessionResult::History {
        cursor: Some(first_cursor),
        ..
    } = first
    else {
        panic!("first page cursor")
    };
    let result = execute(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "case-3".to_owned(),
            },
            context: ContextPolicy::All,
            cursor: Some(first_cursor),
            limit: Some(2),
        },
    )
    .await;
    let SessionResult::History {
        messages, cursor, ..
    } = result
    else {
        panic!("expected history result");
    };
    assert_eq!(
        messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<Vec<_>>(),
        vec!["msg-1", "msg-2"]
    );
    assert!(cursor.is_some());
}

#[tokio::test]
async fn in_memory_session_retention_filters_expired_history() {
    let client = InMemorySessionClient::new();
    resolve(
        &client,
        config_with_retention("case-retention", RetentionPolicy::Days(1)),
    )
    .await;
    append(
        &client,
        SessionMessage {
            created_at: "0".to_owned(),
            ..message("case-retention", "msg-old", "old", None)
        },
    )
    .await;
    append(
        &client,
        SessionMessage {
            created_at: current_unix_seconds_string(),
            ..message("case-retention", "msg-new", "new", None)
        },
    )
    .await;

    let loaded = execute(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "case-retention".to_owned(),
            },
            context: ContextPolicy::All,
            cursor: None,
            limit: None,
        },
    )
    .await;
    let SessionResult::History { messages, .. } = loaded else {
        panic!("expected history result");
    };
    assert_eq!(
        messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<Vec<_>>(),
        vec!["msg-new"]
    );
}

#[tokio::test]
async fn in_memory_session_rejects_append_before_resolve() {
    let client = InMemorySessionClient::new();
    let response = client
        .execute(request(SessionOperation::Append {
            message: message("missing", "msg-0", "payload", None),
        }))
        .await
        .unwrap();
    let err = response.result.expect_err("append must fail closed");
    assert!(err.message.contains("unresolved session"));
}

#[tokio::test]
async fn sqlite_session_persists_history_without_generating_summary() {
    let path = sqlite_session_path("history");
    let first = SqliteSessionClient::open(&path).expect("open first SQLite session client");
    resolve_sqlite(&first, config("sqlite-history")).await;
    append_sqlite(
        &first,
        message(
            "sqlite-history",
            "msg-0",
            "hello",
            Some("turn:0".to_owned()),
        ),
    )
    .await;
    drop(first);

    let second = SqliteSessionClient::open(&path).expect("reopen SQLite session client");
    let loaded = execute_sqlite(
        &second,
        SessionOperation::Load {
            session: SessionRef {
                id: "sqlite-history".to_owned(),
            },
            context: ContextPolicy::SummaryPlusRecent { recent: 1 },
            cursor: None,
            limit: None,
        },
    )
    .await;
    let SessionResult::History {
        messages, summary, ..
    } = loaded
    else {
        panic!("expected history result");
    };
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, "msg-0");
    assert_eq!(messages[0].payload, HostValue::String("hello".to_owned()));
    assert!(summary.is_none());
}

#[tokio::test]
async fn sqlite_session_deduplicates_append_across_clients() {
    let path = sqlite_session_path("dedup");
    let first = SqliteSessionClient::open(&path).expect("open first SQLite session client");
    resolve_sqlite(&first, config("sqlite-dedup")).await;
    append_sqlite(
        &first,
        message(
            "sqlite-dedup",
            "msg-original",
            "first",
            Some("agent-turn:1".to_owned()),
        ),
    )
    .await;
    drop(first);

    let second = SqliteSessionClient::open(&path).expect("reopen SQLite session client");
    let appended = append_sqlite(
        &second,
        message(
            "sqlite-dedup",
            "msg-replay",
            "first",
            Some("agent-turn:1".to_owned()),
        ),
    )
    .await;
    let SessionResult::Appended {
        message,
        deduplicated,
    } = appended
    else {
        panic!("expected append result");
    };
    assert!(deduplicated);
    assert_eq!(message.id, "msg-original");
    assert_eq!(message.payload, HostValue::String("first".to_owned()));
}

#[tokio::test]
async fn sqlite_session_retention_filters_expired_history() {
    let path = sqlite_session_path("retention");
    let client = SqliteSessionClient::open(&path).expect("open SQLite session client");
    resolve_sqlite(
        &client,
        config_with_retention("sqlite-retention", RetentionPolicy::Days(1)),
    )
    .await;
    append_sqlite(
        &client,
        SessionMessage {
            created_at: "1970-01-01T00:00:00Z".to_owned(),
            ..message("sqlite-retention", "msg-old", "old", None)
        },
    )
    .await;
    append_sqlite(
        &client,
        SessionMessage {
            created_at: current_unix_seconds_string(),
            ..message("sqlite-retention", "msg-new", "new", None)
        },
    )
    .await;

    let loaded = execute_sqlite(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "sqlite-retention".to_owned(),
            },
            context: ContextPolicy::All,
            cursor: None,
            limit: None,
        },
    )
    .await;
    let SessionResult::History { messages, .. } = loaded else {
        panic!("expected history result");
    };
    assert_eq!(
        messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<Vec<_>>(),
        vec!["msg-new"]
    );
}

#[tokio::test]
async fn sqlite_session_summary_view_without_publication_returns_recent_history() {
    let path = sqlite_session_path("compact-none");
    let client = SqliteSessionClient::open(&path).expect("open SQLite session client");
    resolve_sqlite(
        &client,
        config_with_retention("sqlite-compact-none", RetentionPolicy::Days(1)),
    )
    .await;
    append_sqlite(
        &client,
        SessionMessage {
            created_at: "1970-01-01T00:00:00Z".to_owned(),
            ..message("sqlite-compact-none", "msg-old", "old", None)
        },
    )
    .await;
    append_sqlite(
        &client,
        SessionMessage {
            created_at: current_unix_seconds_string(),
            ..message("sqlite-compact-none", "msg-new", "new", None)
        },
    )
    .await;

    let compacted = execute_sqlite(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "sqlite-compact-none".to_owned(),
            },
            context: ContextPolicy::SummaryPlusRecent { recent: 1 },
            cursor: None,
            limit: None,
        },
    )
    .await;
    let SessionResult::History {
        messages, summary, ..
    } = compacted
    else {
        panic!("expected unmodified history");
    };
    assert_eq!(messages.len(), 1);
    assert!(summary.is_none());
    assert_eq!(messages[0].payload, HostValue::String("new".into()));
}

async fn resolve(client: &InMemorySessionClient, config: SessionConfig) -> SessionResult {
    execute(client, SessionOperation::Resolve { config }).await
}

#[tokio::test]
async fn session_resolve_and_dedup_reject_conflicting_content() {
    async fn check<C: SessionClient<Error = etas_host::HostError>>(client: &C) {
        client
            .execute(request(SessionOperation::Resolve {
                config: config("conflict"),
            }))
            .await
            .unwrap()
            .result
            .unwrap();
        let result = client
            .execute(request(SessionOperation::Resolve {
                config: config_with_retention("conflict", RetentionPolicy::Forever),
            }))
            .await
            .unwrap()
            .result;
        assert_eq!(
            result.unwrap_err().code,
            etas_host::HostErrorCode::InvalidRequest
        );
        let first = message("conflict", "one", "first", Some("logical-append".into()));
        client
            .execute(request(SessionOperation::Append { message: first }))
            .await
            .unwrap()
            .result
            .unwrap();
        let changed = message("conflict", "two", "changed", Some("logical-append".into()));
        let result = client
            .execute(request(SessionOperation::Append { message: changed }))
            .await
            .unwrap()
            .result;
        assert_eq!(
            result.unwrap_err().code,
            etas_host::HostErrorCode::InvalidRequest
        );
    }
    check(&InMemorySessionClient::new()).await;
    let workspace = etas_host::TestWorkspace::create("session-atomic").unwrap();
    check(&SqliteSessionClient::open(workspace.path().join("db")).unwrap()).await;
}

#[tokio::test]
async fn sqlite_session_ordinal_does_not_reset_after_message_deletion() {
    let workspace = etas_host::TestWorkspace::create("session-ordinal").unwrap();
    let path = workspace.path().join("db");
    let client = SqliteSessionClient::open(&path).unwrap();
    resolve_sqlite(&client, config("ordinal")).await;
    append_sqlite(&client, message("ordinal", "one", "first", None)).await;
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute("DELETE FROM session_messages", [])
        .unwrap();
    append_sqlite(&client, message("ordinal", "two", "second", None)).await;
    let ordinal: i64 = connection
        .query_row("SELECT ordinal FROM session_messages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(ordinal, 1);
}

async fn append(client: &InMemorySessionClient, message: SessionMessage) -> SessionResult {
    execute(client, SessionOperation::Append { message }).await
}

async fn execute(client: &InMemorySessionClient, operation: SessionOperation) -> SessionResult {
    client
        .execute(request(operation))
        .await
        .unwrap()
        .result
        .unwrap()
}

async fn resolve_sqlite(client: &SqliteSessionClient, config: SessionConfig) -> SessionResult {
    execute_sqlite(client, SessionOperation::Resolve { config }).await
}

async fn append_sqlite(client: &SqliteSessionClient, message: SessionMessage) -> SessionResult {
    execute_sqlite(client, SessionOperation::Append { message }).await
}

async fn execute_sqlite(
    client: &SqliteSessionClient,
    operation: SessionOperation,
) -> SessionResult {
    client
        .execute(request(operation))
        .await
        .unwrap()
        .result
        .unwrap()
}

#[tokio::test]
async fn sqlite_session_reads_use_configured_limits_for_history_id_and_dedup() {
    let path = sqlite_session_path("read-limits");
    let writer = SqliteSessionClient::open(&path).unwrap();
    execute_sqlite(
        &writer,
        SessionOperation::Resolve {
            config: config_with_retention("limits", RetentionPolicy::Forever),
        },
    )
    .await;
    append_sqlite(
        &writer,
        message(
            "limits",
            "large",
            &"x".repeat(1024),
            Some("dedup".to_owned()),
        ),
    )
    .await;
    let reader = SqliteSessionClient::open_with_limits(
        &path,
        etas_host::StorageLimits {
            max_value_bytes: 512,
            ..Default::default()
        },
    )
    .unwrap();
    for operation in [
        SessionOperation::Load {
            session: SessionRef {
                id: "limits".to_owned(),
            },
            context: ContextPolicy::All,
            cursor: None,
            limit: Some(1),
        },
        SessionOperation::Append {
            message: message("limits", "large", "small", None),
        },
        SessionOperation::Append {
            message: message("limits", "another", "small", Some("dedup".to_owned())),
        },
    ] {
        let error = reader
            .execute(request(operation))
            .await
            .unwrap()
            .result
            .unwrap_err();
        assert_eq!(
            error.code,
            etas_host::HostErrorCode::BudgetExceeded,
            "{error:?}"
        );
    }
}

#[tokio::test]
async fn sqlite_session_codec_honors_limits_larger_than_defaults() {
    let path = sqlite_session_path("large-value-limits");
    let client = SqliteSessionClient::open_with_limits(
        &path,
        etas_host::StorageLimits {
            max_value_bytes: 2 * 1024 * 1024,
            ..Default::default()
        },
    )
    .unwrap();
    execute_sqlite(
        &client,
        SessionOperation::Resolve {
            config: config_with_retention("large-limits", RetentionPolicy::Forever),
        },
    )
    .await;
    let original = message(
        "large-limits",
        "large",
        &"x".repeat(1024 * 1024 + 1),
        Some("dedup".to_owned()),
    );
    append_sqlite(&client, original.clone()).await;
    let result = execute_sqlite(
        &client,
        SessionOperation::Load {
            session: original.session.clone(),
            context: ContextPolicy::All,
            cursor: None,
            limit: Some(1),
        },
    )
    .await;
    let SessionResult::History { messages, .. } = result else {
        panic!("history")
    };
    assert_eq!(messages, vec![original.clone()]);
    let result = append_sqlite(&client, original).await;
    assert!(matches!(
        result,
        SessionResult::Appended {
            deduplicated: true,
            ..
        }
    ));
}

fn request(operation: SessionOperation) -> SessionRequest {
    SessionRequest {
        id: HostRequestId(1),
        operation,
        authority: AuthorityContext::deny_all(),
        trace: TraceContext::root(TraceId(1)),
        budget: etas_host::ExecutionBudget::default(),
    }
}

fn config(id: &str) -> SessionConfig {
    config_with_retention(id, RetentionPolicy::Days(90))
}

fn config_with_retention(id: &str, retention: RetentionPolicy) -> SessionConfig {
    SessionConfig {
        id: id.to_owned(),
        context: ContextPolicy::SummaryPlusRecent { recent: 4 },
        retention,
    }
}

fn message(session: &str, id: &str, payload: &str, dedup_key: Option<String>) -> SessionMessage {
    SessionMessage {
        id: id.to_owned(),
        from: None,
        to: None,
        role: SessionMessageRole::User,
        session: SessionRef {
            id: session.to_owned(),
        },
        created_at: "2026-06-19T00:00:00Z".to_owned(),
        payload: HostValue::String(payload.to_owned()),
        provenance: None,
        dedup_key,
    }
}

fn sqlite_session_path(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "etas-host-session-{name}-{}-{nanos}.sqlite",
        std::process::id()
    ))
}

fn current_unix_seconds_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
        .to_string()
}
