use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::{Arc, RwLock},
};

use crate::{
    CompactionPolicy, ContextPolicy, HostError, HostErrorCode, SessionClient, SessionConfig,
    SessionCursor, SessionMessage, SessionOperation, SessionRef, SessionRequest, SessionResponse,
    SessionResult, SessionSummary,
};

use super::retention::retain_messages;

#[derive(Clone, Debug, Default)]
pub struct InMemorySessionClient {
    sessions: Arc<RwLock<BTreeMap<String, SessionState>>>,
}

#[derive(Clone, Debug)]
struct SessionState {
    config: SessionConfig,
    messages: Vec<SessionMessage>,
    dedup: BTreeMap<String, String>,
    summary: Option<SessionSummary>,
}

impl InMemorySessionClient {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionClient for InMemorySessionClient {
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

impl InMemorySessionClient {
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
        let mut sessions = self.sessions.write().map_err(lock_error)?;
        let created = !sessions.contains_key(&config.id);
        sessions
            .entry(config.id.clone())
            .and_modify(|state| state.config = config.clone())
            .or_insert_with(|| SessionState {
                config: config.clone(),
                messages: Vec::new(),
                dedup: BTreeMap::new(),
                summary: None,
            });
        Ok(SessionResult::Resolved {
            session: SessionRef { id: config.id },
            created,
        })
    }

    fn append(&self, message: SessionMessage) -> Result<SessionResult, HostError> {
        if message.id.is_empty() {
            return Err(invalid_request("message id must not be empty"));
        }
        if message.session.id.is_empty() {
            return Err(invalid_request("message session id must not be empty"));
        }
        let mut sessions = self.sessions.write().map_err(lock_error)?;
        let Some(state) = sessions.get_mut(&message.session.id) else {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "cannot append message to an unresolved session",
            )
            .with_detail("session", message.session.id));
        };
        if let Some(dedup_key) = &message.dedup_key
            && let Some(existing_id) = state.dedup.get(dedup_key)
            && let Some(existing) = state
                .messages
                .iter()
                .find(|candidate| &candidate.id == existing_id)
                .cloned()
        {
            return Ok(SessionResult::Appended {
                message: existing,
                deduplicated: true,
            });
        }
        if state
            .messages
            .iter()
            .any(|candidate| candidate.id == message.id)
        {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "session message id already exists",
            )
            .with_detail("session", message.session.id)
            .with_detail("message", message.id));
        }
        if let Some(dedup_key) = &message.dedup_key {
            state.dedup.insert(dedup_key.clone(), message.id.clone());
        }
        state.messages.push(message.clone());
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
        let sessions = self.sessions.read().map_err(lock_error)?;
        let Some(state) = sessions.get(&session.id) else {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "cannot load unresolved session history",
            )
            .with_detail("session", session.id));
        };
        let start = cursor
            .as_ref()
            .map(|cursor| parse_cursor(cursor, state.messages.len()))
            .transpose()?
            .unwrap_or(0);
        let retained = retain_messages(&state.messages, &state.config.retention)?;
        let selected = select_context(&retained, &context);
        let mut messages = selected.into_iter().skip(start).collect::<Vec<_>>();
        let limit = limit.map(|limit| limit as usize);
        let next_cursor = if let Some(limit) = limit.filter(|limit| messages.len() > *limit) {
            messages.truncate(limit);
            Some(SessionCursor {
                opaque: (start + limit).to_string(),
            })
        } else {
            None
        };
        let summary = match context {
            ContextPolicy::SummaryPlusRecent { .. } => state.summary.clone(),
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
        let mut sessions = self.sessions.write().map_err(lock_error)?;
        let Some(state) = sessions.get_mut(&session.id) else {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "cannot compact unresolved session history",
            )
            .with_detail("session", session.id));
        };
        match policy {
            CompactionPolicy::None => {
                let retained = retain_messages(&state.messages, &state.config.retention)?;
                let summary = state.summary.clone().unwrap_or_else(|| SessionSummary {
                    text: String::new(),
                    message_count: retained.len(),
                });
                Ok(SessionResult::Compacted { session, summary })
            }
            CompactionPolicy::SummarizeWhen { max_context_tokens } => {
                if max_context_tokens == 0 {
                    return Err(invalid_request(
                        "session compaction token budget must be nonzero",
                    ));
                }
                let retained = retain_messages(&state.messages, &state.config.retention)?;
                let summary = summarize_messages(&retained);
                state.summary = Some(summary.clone());
                Ok(SessionResult::Compacted { session, summary })
            }
        }
    }
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
    let mut participants = BTreeSet::new();
    for message in messages {
        if let Some(from) = &message.from {
            participants.insert(from.clone());
        }
        if let Some(to) = &message.to {
            participants.insert(to.clone());
        }
    }
    let text = if participants.is_empty() {
        format!("{} message(s)", messages.len())
    } else {
        format!(
            "{} message(s); participants: {}",
            messages.len(),
            participants.into_iter().collect::<Vec<_>>().join(", ")
        )
    };
    SessionSummary {
        text,
        message_count: messages.len(),
    }
}

fn parse_cursor(cursor: &SessionCursor, len: usize) -> Result<usize, HostError> {
    let value = cursor.opaque.parse::<usize>().map_err(|_| {
        HostError::new(
            HostErrorCode::InvalidRequest,
            "invalid session history cursor",
        )
        .with_detail("cursor", cursor.opaque.clone())
    })?;
    if value > len {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "session history cursor is beyond the end of the history",
        )
        .with_detail("cursor", cursor.opaque.clone())
        .with_detail("message_count", len.to_string()));
    }
    Ok(value)
}

fn invalid_request(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, message)
}

fn lock_error<T>(_: T) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "session store lock poisoned",
    )
}
