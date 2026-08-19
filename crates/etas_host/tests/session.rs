use etas_host::{
    AuthorityContext, CompactionPolicy, ContextPolicy, HostRequestId, HostValue,
    InMemorySessionClient, RetentionPolicy, SessionClient, SessionConfig, SessionCursor,
    SessionMessage, SessionMessageRole, SessionOperation, SessionRef, SessionRequest,
    SessionResult, SqliteSessionClient, TraceContext, TraceId,
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
        message("case-2", "msg-b", "second", Some("turn:1".to_owned())),
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

    let result = execute(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "case-3".to_owned(),
            },
            context: ContextPolicy::All,
            cursor: Some(SessionCursor {
                opaque: "1".to_owned(),
            }),
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
    assert_eq!(cursor.expect("next cursor").opaque, "3");
}

#[tokio::test]
async fn in_memory_session_compacts_and_returns_summary_with_recent_history() {
    let client = InMemorySessionClient::new();
    resolve(&client, config("case-4")).await;
    append(
        &client,
        SessionMessage {
            from: Some("customer".to_owned()),
            to: Some("triage".to_owned()),
            ..message("case-4", "msg-0", "hello", None)
        },
    )
    .await;
    append(
        &client,
        SessionMessage {
            from: Some("triage".to_owned()),
            to: Some("customer".to_owned()),
            ..message("case-4", "msg-1", "reply", None)
        },
    )
    .await;

    let compacted = execute(
        &client,
        SessionOperation::Compact {
            session: SessionRef {
                id: "case-4".to_owned(),
            },
            policy: CompactionPolicy::SummarizeWhen {
                max_context_tokens: 1024,
            },
        },
    )
    .await;
    let SessionResult::Compacted { summary, .. } = compacted else {
        panic!("expected compacted result");
    };
    assert_eq!(summary.message_count, 2);
    assert!(summary.text.contains("customer"));

    let loaded = execute(
        &client,
        SessionOperation::Load {
            session: SessionRef {
                id: "case-4".to_owned(),
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
    assert_eq!(messages.len(), 2);
    assert_eq!(summary.expect("summary").message_count, 2);
}

#[tokio::test]
async fn in_memory_session_retention_filters_expired_history_and_compaction() {
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

    let compacted = execute(
        &client,
        SessionOperation::Compact {
            session: SessionRef {
                id: "case-retention".to_owned(),
            },
            policy: CompactionPolicy::SummarizeWhen {
                max_context_tokens: 1024,
            },
        },
    )
    .await;
    let SessionResult::Compacted { summary, .. } = compacted else {
        panic!("expected compacted result");
    };
    assert_eq!(summary.message_count, 1);
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
async fn sqlite_session_persists_history_and_summary_across_clients() {
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
    execute_sqlite(
        &first,
        SessionOperation::Compact {
            session: SessionRef {
                id: "sqlite-history".to_owned(),
            },
            policy: CompactionPolicy::SummarizeWhen {
                max_context_tokens: 1024,
            },
        },
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
    assert_eq!(summary.expect("summary").message_count, 1);
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
            "second",
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
async fn sqlite_session_compaction_none_reports_retained_history_count() {
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
        SessionOperation::Compact {
            session: SessionRef {
                id: "sqlite-compact-none".to_owned(),
            },
            policy: CompactionPolicy::None,
        },
    )
    .await;
    let SessionResult::Compacted { summary, .. } = compacted else {
        panic!("expected compacted result");
    };
    assert_eq!(summary.message_count, 1);
    assert!(summary.text.contains("new"));
}

async fn resolve(client: &InMemorySessionClient, config: SessionConfig) -> SessionResult {
    execute(client, SessionOperation::Resolve { config }).await
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
        compaction: CompactionPolicy::SummarizeWhen {
            max_context_tokens: 24_000,
        },
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
