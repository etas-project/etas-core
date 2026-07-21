use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{Arc, Mutex},
};

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::{
    CompactionPolicy, ContextPolicy, HostError, HostErrorCode, HostValue, RetentionPolicy,
    SessionClient, SessionConfig, SessionCursor, SessionMessage, SessionMessageRole,
    SessionOperation, SessionRef, SessionRequest, SessionResponse, SessionResult, SessionSummary,
    value::tagged::{host_value_from_tagged_json_str, host_value_to_tagged_json_string},
};

use super::retention::retain_messages;

#[derive(Clone, Debug)]
pub struct SqliteSessionClient {
    connection: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl SqliteSessionClient {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, HostError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent).map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "failed to create SQLite session directory",
                )
                .with_detail("path", parent.display().to_string())
                .with_detail("error", error.to_string())
            })?;
        }
        let connection = Connection::open(&path).map_err(sqlite_error)?;
        initialize_schema(&connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            path,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SessionClient for SqliteSessionClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<SessionResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: SessionRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            let id = request.id;
            let result = self.execute_operation(request.operation);
            Ok(SessionResponse { id, result })
        })
    }
}

impl SqliteSessionClient {
    fn execute_operation(&self, operation: SessionOperation) -> Result<SessionResult, HostError> {
        match operation {
            SessionOperation::Resolve { config } => self.resolve(config),
            SessionOperation::Append { message } => self.append(message),
            SessionOperation::Load {
                session,
                context,
                cursor,
                limit,
            } => self.load(session, context, cursor, limit),
            SessionOperation::Compact { session, policy } => self.compact(session, policy),
        }
    }

    fn resolve(&self, config: SessionConfig) -> Result<SessionResult, HostError> {
        if config.id.is_empty() {
            return Err(invalid_request("session id must not be empty"));
        }
        let config_json = session_config_json(&config)?.to_string();
        let connection = self.connection.lock().map_err(lock_error)?;
        let existed = session_exists(&connection, &config.id)?;
        connection
            .execute(
                "INSERT INTO sessions (id, config_json, summary_text, summary_message_count)
                 VALUES (?1, ?2, NULL, NULL)
                 ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json",
                params![config.id, config_json],
            )
            .map_err(sqlite_error)?;
        Ok(SessionResult::Resolved {
            session: SessionRef { id: config.id },
            created: !existed,
        })
    }

    fn append(&self, message: SessionMessage) -> Result<SessionResult, HostError> {
        if message.id.is_empty() {
            return Err(invalid_request("message id must not be empty"));
        }
        if message.session.id.is_empty() {
            return Err(invalid_request("message session id must not be empty"));
        }
        let payload_json = host_value_to_tagged_json_string(&message.payload)?;
        let provenance_json = message
            .provenance
            .as_ref()
            .map(host_value_to_tagged_json_string)
            .transpose()?
            .map(|value| value.to_string());
        let mut connection = self.connection.lock().map_err(lock_error)?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        if !session_exists(&transaction, &message.session.id)? {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "cannot append message to an unresolved session",
            )
            .with_detail("session", message.session.id));
        }
        if let Some(dedup_key) = &message.dedup_key
            && let Some(existing) =
                select_message_by_dedup(&transaction, &message.session.id, dedup_key)?
        {
            return Ok(SessionResult::Appended {
                message: existing,
                deduplicated: true,
            });
        }
        if select_message(&transaction, &message.session.id, &message.id)?.is_some() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "session message id already exists",
            )
            .with_detail("session", message.session.id)
            .with_detail("message", message.id));
        }
        let ordinal = next_ordinal(&transaction, &message.session.id)?;
        transaction
            .execute(
                "INSERT INTO session_messages
                    (session_id, message_id, ordinal, role, from_participant, to_participant,
                     created_at, payload_json, provenance_json, dedup_key)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    message.session.id,
                    message.id,
                    ordinal,
                    role_name(message.role),
                    message.from,
                    message.to,
                    message.created_at,
                    payload_json,
                    provenance_json,
                    message.dedup_key,
                ],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(SessionResult::Appended {
            message,
            deduplicated: false,
        })
    }

    fn load(
        &self,
        session: SessionRef,
        context: ContextPolicy,
        cursor: Option<SessionCursor>,
        limit: Option<u32>,
    ) -> Result<SessionResult, HostError> {
        let connection = self.connection.lock().map_err(lock_error)?;
        if !session_exists(&connection, &session.id)? {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "cannot load unresolved session history",
            )
            .with_detail("session", session.id));
        }
        let offset = cursor
            .as_ref()
            .map(|cursor| {
                cursor.opaque.parse::<usize>().map_err(|error| {
                    HostError::new(HostErrorCode::InvalidRequest, "invalid session cursor")
                        .with_detail("cursor", &cursor.opaque)
                        .with_detail("error", error.to_string())
                })
            })
            .transpose()?
            .unwrap_or(0);
        let retention = select_retention_policy(&connection, &session.id)?;
        let all_messages = select_messages(&connection, &session.id)?;
        let retained_messages = retain_messages(&all_messages, &retention)?;
        let selected = select_context(&retained_messages, &context);
        let mut messages = selected.into_iter().skip(offset).collect::<Vec<_>>();
        let limit = limit.map(|limit| limit as usize);
        let next_cursor = if let Some(limit) = limit.filter(|limit| messages.len() > *limit) {
            messages.truncate(limit);
            Some(SessionCursor {
                opaque: (offset + limit).to_string(),
            })
        } else {
            None
        };
        let summary = match context {
            ContextPolicy::SummaryPlusRecent { .. } => select_summary(&connection, &session.id)?,
            ContextPolicy::All | ContextPolicy::LastTurns(_) => None,
        };
        Ok(SessionResult::History {
            session,
            messages,
            summary,
            cursor: next_cursor,
        })
    }

    fn compact(
        &self,
        session: SessionRef,
        policy: CompactionPolicy,
    ) -> Result<SessionResult, HostError> {
        let connection = self.connection.lock().map_err(lock_error)?;
        if !session_exists(&connection, &session.id)? {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "cannot compact unresolved session history",
            )
            .with_detail("session", session.id));
        }
        let summary = match policy {
            CompactionPolicy::None => match select_summary(&connection, &session.id)? {
                Some(summary) => summary,
                None => {
                    let retention = select_retention_policy(&connection, &session.id)?;
                    let messages = select_messages(&connection, &session.id)?;
                    let retained_messages = retain_messages(&messages, &retention)?;
                    summarize_messages(&retained_messages)
                }
            },
            CompactionPolicy::SummarizeWhen { max_context_tokens } => {
                if max_context_tokens == 0 {
                    return Err(invalid_request(
                        "session compaction token budget must be nonzero",
                    ));
                }
                let retention = select_retention_policy(&connection, &session.id)?;
                let messages = select_messages(&connection, &session.id)?;
                let retained_messages = retain_messages(&messages, &retention)?;
                summarize_messages(&retained_messages)
            }
        };
        connection
            .execute(
                "UPDATE sessions
                 SET summary_text = ?2, summary_message_count = ?3
                 WHERE id = ?1",
                params![
                    session.id,
                    summary.text,
                    i64::try_from(summary.message_count).map_err(count_error)?,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(SessionResult::Compacted { session, summary })
    }
}

fn initialize_schema(connection: &Connection) -> Result<(), HostError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                config_json TEXT NOT NULL,
                summary_text TEXT,
                summary_message_count INTEGER
            );
            CREATE TABLE IF NOT EXISTS session_messages (
                session_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                role TEXT NOT NULL,
                from_participant TEXT,
                to_participant TEXT,
                created_at TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                provenance_json TEXT,
                dedup_key TEXT,
                PRIMARY KEY(session_id, message_id),
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_session_messages_dedup
                ON session_messages(session_id, dedup_key)
                WHERE dedup_key IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_session_messages_order
                ON session_messages(session_id, ordinal);",
        )
        .map_err(sqlite_error)
}

fn session_exists(connection: &Connection, session: &str) -> Result<bool, HostError> {
    connection
        .query_row(
            "SELECT 1 FROM sessions WHERE id = ?1",
            params![session],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(sqlite_error)
}

fn select_retention_policy(
    connection: &Connection,
    session: &str,
) -> Result<RetentionPolicy, HostError> {
    let config_json = connection
        .query_row(
            "SELECT config_json
             FROM sessions
             WHERE id = ?1",
            params![session],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sqlite_error)?
        .ok_or_else(|| {
            HostError::new(HostErrorCode::InvalidRequest, "session does not exist")
                .with_detail("session", session.to_owned())
        })?;
    let config = serde_json::from_str::<Value>(&config_json).map_err(json_error)?;
    let retention = config
        .get("retention")
        .ok_or_else(|| invalid_request("session config is missing retention policy"))?;
    retention_policy_from_json(retention)
}

fn next_ordinal(connection: &Connection, session: &str) -> Result<i64, HostError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(ordinal), -1) + 1
             FROM session_messages
             WHERE session_id = ?1",
            params![session],
            |row| row.get::<_, i64>(0),
        )
        .map_err(sqlite_error)
}

fn select_message(
    connection: &Connection,
    session: &str,
    message: &str,
) -> Result<Option<SessionMessage>, HostError> {
    let mut statement = connection
        .prepare(
            "SELECT message_id, role, from_participant, to_participant, created_at,
                    payload_json, provenance_json, dedup_key
             FROM session_messages
             WHERE session_id = ?1 AND message_id = ?2",
        )
        .map_err(sqlite_error)?;
    statement
        .query_row(params![session, message], stored_message_from_row)
        .optional()
        .map_err(sqlite_error)?
        .map(|message| decode_message(session, message))
        .transpose()
}

fn select_message_by_dedup(
    connection: &Connection,
    session: &str,
    dedup_key: &str,
) -> Result<Option<SessionMessage>, HostError> {
    let mut statement = connection
        .prepare(
            "SELECT message_id, role, from_participant, to_participant, created_at,
                    payload_json, provenance_json, dedup_key
             FROM session_messages
             WHERE session_id = ?1 AND dedup_key = ?2",
        )
        .map_err(sqlite_error)?;
    statement
        .query_row(params![session, dedup_key], stored_message_from_row)
        .optional()
        .map_err(sqlite_error)?
        .map(|message| decode_message(session, message))
        .transpose()
}

fn select_messages(
    connection: &Connection,
    session: &str,
) -> Result<Vec<SessionMessage>, HostError> {
    let mut statement = connection
        .prepare(
            "SELECT message_id, role, from_participant, to_participant, created_at,
                    payload_json, provenance_json, dedup_key
             FROM session_messages
             WHERE session_id = ?1
             ORDER BY ordinal",
        )
        .map_err(sqlite_error)?;
    let rows = statement
        .query_map(params![session], stored_message_from_row)
        .map_err(sqlite_error)?;
    let mut messages = Vec::new();
    for row in rows {
        messages.push(decode_message(session, row.map_err(sqlite_error)?)?);
    }
    Ok(messages)
}

struct StoredSessionMessage {
    id: String,
    role: String,
    from: Option<String>,
    to: Option<String>,
    created_at: String,
    payload_json: String,
    provenance_json: Option<String>,
    dedup_key: Option<String>,
}

fn stored_message_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredSessionMessage> {
    Ok(StoredSessionMessage {
        id: row.get(0)?,
        role: row.get(1)?,
        from: row.get(2)?,
        to: row.get(3)?,
        created_at: row.get(4)?,
        payload_json: row.get(5)?,
        provenance_json: row.get(6)?,
        dedup_key: row.get(7)?,
    })
}

fn decode_message(
    session: &str,
    message: StoredSessionMessage,
) -> Result<SessionMessage, HostError> {
    Ok(SessionMessage {
        id: message.id,
        role: role_from_name(&message.role)
            .map_err(|error| HostError::new(HostErrorCode::InvalidRequest, error))?,
        from: message.from,
        to: message.to,
        session: SessionRef {
            id: session.to_owned(),
        },
        created_at: message.created_at,
        payload: json_to_host_value(&message.payload_json)?,
        provenance: message
            .provenance_json
            .as_deref()
            .map(json_to_host_value)
            .transpose()?,
        dedup_key: message.dedup_key,
    })
}

fn select_summary(
    connection: &Connection,
    session: &str,
) -> Result<Option<SessionSummary>, HostError> {
    connection
        .query_row(
            "SELECT summary_text, summary_message_count
             FROM sessions
             WHERE id = ?1",
            params![session],
            |row| {
                let text: Option<String> = row.get(0)?;
                let count: Option<i64> = row.get(1)?;
                Ok(text.zip(count))
            },
        )
        .optional()
        .map_err(sqlite_error)?
        .flatten()
        .map(|(text, count)| {
            Ok(SessionSummary {
                text,
                message_count: usize::try_from(count).map_err(count_error)?,
            })
        })
        .transpose()
}

fn select_context(messages: &[SessionMessage], context: &ContextPolicy) -> Vec<SessionMessage> {
    match context {
        ContextPolicy::All => messages.to_vec(),
        ContextPolicy::LastTurns(turns) | ContextPolicy::SummaryPlusRecent { recent: turns } => {
            let message_count = turns.saturating_mul(2);
            let start = messages.len().saturating_sub(message_count);
            messages[start..].to_vec()
        }
    }
}

fn summarize_messages(messages: &[SessionMessage]) -> SessionSummary {
    let text = messages
        .iter()
        .map(|message| format!("{}: {:?}", role_name(message.role), message.payload))
        .collect::<Vec<_>>()
        .join("\n");
    SessionSummary {
        text,
        message_count: messages.len(),
    }
}

fn session_config_json(config: &SessionConfig) -> Result<Value, HostError> {
    Ok(json!({
        "id": config.id,
        "context": context_policy_json(&config.context),
        "retention": retention_policy_json(&config.retention),
        "compaction": compaction_policy_json(&config.compaction),
    }))
}

fn context_policy_json(policy: &ContextPolicy) -> Value {
    match policy {
        ContextPolicy::All => json!({ "kind": "All" }),
        ContextPolicy::LastTurns(turns) => json!({ "kind": "LastTurns", "turns": turns }),
        ContextPolicy::SummaryPlusRecent { recent } => {
            json!({ "kind": "SummaryPlusRecent", "recent": recent })
        }
    }
}

fn retention_policy_json(policy: &RetentionPolicy) -> Value {
    match policy {
        RetentionPolicy::Forever => json!({ "kind": "Forever" }),
        RetentionPolicy::Days(days) => json!({ "kind": "Days", "days": days }),
    }
}

fn retention_policy_from_json(value: &Value) -> Result<RetentionPolicy, HostError> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_request("session retention policy is missing kind"))?;
    match kind {
        "Forever" => Ok(RetentionPolicy::Forever),
        "Days" => {
            let days = value
                .get("days")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_request("session retention policy is missing days"))?;
            Ok(RetentionPolicy::Days(days))
        }
        other => Err(invalid_request(format!(
            "unknown session retention policy `{other}`"
        ))),
    }
}

fn compaction_policy_json(policy: &CompactionPolicy) -> Value {
    match policy {
        CompactionPolicy::None => json!({ "kind": "None" }),
        CompactionPolicy::SummarizeWhen { max_context_tokens } => {
            json!({ "kind": "SummarizeWhen", "max_context_tokens": max_context_tokens })
        }
    }
}

fn json_to_host_value(value: &str) -> Result<HostValue, HostError> {
    host_value_from_tagged_json_str(value)
}

fn role_name(role: SessionMessageRole) -> &'static str {
    match role {
        SessionMessageRole::System => "system",
        SessionMessageRole::User => "user",
        SessionMessageRole::Assistant => "assistant",
        SessionMessageRole::Tool => "tool",
    }
}

fn role_from_name(value: &str) -> Result<SessionMessageRole, String> {
    match value {
        "system" => Ok(SessionMessageRole::System),
        "user" => Ok(SessionMessageRole::User),
        "assistant" => Ok(SessionMessageRole::Assistant),
        "tool" => Ok(SessionMessageRole::Tool),
        other => Err(format!("unknown session message role `{other}`")),
    }
}

fn invalid_request(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, message)
}

fn sqlite_error(error: rusqlite::Error) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, "SQLite session error")
        .with_detail("error", error.to_string())
}

fn json_error(error: serde_json::Error) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "invalid SQLite session JSON")
        .with_detail("error", error.to_string())
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "SQLite session lock poisoned",
    )
    .with_detail("error", error.to_string())
}

fn count_error(error: impl std::fmt::Display) -> HostError {
    HostError::new(
        HostErrorCode::SchemaMismatch,
        "session count cannot be represented",
    )
    .with_detail("error", error.to_string())
}
