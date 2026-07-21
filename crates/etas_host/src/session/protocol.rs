use crate::{AuthorityContext, Budget, HostError, HostRequestId, HostValue, TraceContext};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionRef {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionConfig {
    pub id: String,
    pub context: ContextPolicy,
    pub retention: RetentionPolicy,
    pub compaction: CompactionPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextPolicy {
    All,
    LastTurns(usize),
    SummaryPlusRecent { recent: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetentionPolicy {
    Forever,
    Days(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompactionPolicy {
    None,
    SummarizeWhen { max_context_tokens: u64 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionMessage {
    pub id: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub role: SessionMessageRole,
    pub session: SessionRef,
    pub created_at: String,
    pub payload: HostValue,
    pub provenance: Option<HostValue>,
    pub dedup_key: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionMessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionRequest {
    pub id: HostRequestId,
    pub operation: SessionOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionOperation {
    Resolve {
        config: SessionConfig,
    },
    Append {
        message: SessionMessage,
    },
    Load {
        session: SessionRef,
        context: ContextPolicy,
        cursor: Option<SessionCursor>,
        limit: Option<u32>,
    },
    Compact {
        session: SessionRef,
        policy: CompactionPolicy,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionResponse {
    pub id: HostRequestId,
    pub result: Result<SessionResult, HostError>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionResult {
    Resolved {
        session: SessionRef,
        created: bool,
    },
    Appended {
        message: SessionMessage,
        deduplicated: bool,
    },
    History {
        session: SessionRef,
        messages: Vec<SessionMessage>,
        summary: Option<SessionSummary>,
        cursor: Option<SessionCursor>,
    },
    Compacted {
        session: SessionRef,
        summary: SessionSummary,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionCursor {
    pub opaque: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionSummary {
    pub text: String,
    pub message_count: usize,
}
