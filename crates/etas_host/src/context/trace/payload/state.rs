use crate::{
    CompactionPolicy, ContextPolicy, HostTraceFieldSensitivity, HostTracePayload, HostValue,
    MemoryOperation, MemoryQuery, MemoryRequest, MemoryVersion, MemoryWriteMode, RetentionPolicy,
    SessionConfig, SessionMessage, SessionMessageRole, SessionOperation, SessionRequest,
};

use super::{HostTraceRequest, option, record, strings, variant};

impl HostTraceRequest for MemoryRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            MemoryOperation::Get { key } => ("Memory.read", variant("Get", vec![key.clone()])),
            MemoryOperation::Put {
                key,
                value,
                expected,
                mode,
            } => (
                "Memory.write",
                variant(
                    "Put",
                    vec![
                        key.clone(),
                        value.clone(),
                        option(expected.as_ref().map(version)),
                        HostValue::String(write_mode_name(*mode).to_owned()),
                    ],
                ),
            ),
            MemoryOperation::Delete { key, expected } => (
                "Memory.write",
                variant(
                    "Delete",
                    vec![key.clone(), option(expected.as_ref().map(version))],
                ),
            ),
            MemoryOperation::Scan { cursor, limit } => (
                "Memory.read",
                variant(
                    "Scan",
                    vec![
                        option(
                            cursor
                                .as_ref()
                                .map(|cursor| HostValue::String(cursor.opaque.clone())),
                        ),
                        option(limit.map(|limit| HostValue::UInt(limit as u128))),
                    ],
                ),
            ),
            MemoryOperation::Query { query, limit } => (
                "Memory.read",
                variant(
                    "Query",
                    vec![
                        query_value(query),
                        option(limit.map(|limit| HostValue::UInt(limit as u128))),
                    ],
                ),
            ),
            MemoryOperation::VectorSearch {
                embedding,
                limit,
                filter,
            } => (
                "Memory.read",
                variant(
                    "VectorSearch",
                    vec![
                        HostValue::List(
                            embedding
                                .iter()
                                .map(|value| HostValue::Float(*value as f64))
                                .collect(),
                        ),
                        HostValue::UInt(*limit as u128),
                        option(filter.clone()),
                    ],
                ),
            ),
        };
        HostTracePayload::new("memory", action)
            .with_field(
                "store",
                record([
                    (
                        "region",
                        HostValue::String(self.store.region.stable_id.clone()),
                    ),
                    (
                        "schema_fingerprint",
                        option(
                            self.store
                                .region
                                .schema_fingerprint
                                .clone()
                                .map(HostValue::String),
                        ),
                    ),
                    ("path", strings(&self.store.path)),
                ]),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field("operation", operation, HostTraceFieldSensitivity::Sensitive)
    }
}

impl HostTraceRequest for SessionRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            SessionOperation::Resolve { config } => (
                "Session.resolve",
                variant("Resolve", vec![config_value(config)]),
            ),
            SessionOperation::Append { message } => (
                "Session.append",
                variant("Append", vec![message_value(message)]),
            ),
            SessionOperation::Load {
                session,
                context,
                cursor,
                limit,
            } => (
                "Session.load",
                variant(
                    "Load",
                    vec![
                        HostValue::String(session.id.clone()),
                        context_value(context),
                        option(
                            cursor
                                .as_ref()
                                .map(|cursor| HostValue::String(cursor.opaque.clone())),
                        ),
                        option(limit.map(|limit| HostValue::UInt(limit as u128))),
                    ],
                ),
            ),
            SessionOperation::Compact { session, policy } => (
                "Session.compact",
                variant(
                    "Compact",
                    vec![
                        HostValue::String(session.id.clone()),
                        compaction_value(policy),
                    ],
                ),
            ),
        };
        HostTracePayload::new("session", action).with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

fn version(value: &MemoryVersion) -> HostValue {
    HostValue::String(value.opaque.clone())
}

fn write_mode_name(mode: MemoryWriteMode) -> &'static str {
    match mode {
        MemoryWriteMode::Put => "put",
        MemoryWriteMode::Insert => "insert",
        MemoryWriteMode::Update => "update",
        MemoryWriteMode::Upsert => "upsert",
    }
}

fn query_value(query: &MemoryQuery) -> HostValue {
    record([
        ("predicate", option(query.predicate.clone())),
        (
            "order_by",
            HostValue::List(
                query
                    .order_by
                    .iter()
                    .map(|key| {
                        record([
                            ("field_path", strings(&key.field_path)),
                            ("descending", HostValue::Bool(key.descending)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn config_value(config: &SessionConfig) -> HostValue {
    record([
        ("id", HostValue::String(config.id.clone())),
        ("context", context_value(&config.context)),
        ("retention", retention_value(&config.retention)),
        ("compaction", compaction_value(&config.compaction)),
    ])
}

fn context_value(policy: &ContextPolicy) -> HostValue {
    match policy {
        ContextPolicy::All => variant("All", Vec::new()),
        ContextPolicy::LastTurns(turns) => {
            variant("LastTurns", vec![HostValue::UInt(*turns as u128)])
        }
        ContextPolicy::SummaryPlusRecent { recent } => {
            variant("SummaryPlusRecent", vec![HostValue::UInt(*recent as u128)])
        }
    }
}

fn retention_value(policy: &RetentionPolicy) -> HostValue {
    match policy {
        RetentionPolicy::Forever => variant("Forever", Vec::new()),
        RetentionPolicy::Days(days) => variant("Days", vec![HostValue::UInt(*days as u128)]),
    }
}

fn compaction_value(policy: &CompactionPolicy) -> HostValue {
    match policy {
        CompactionPolicy::None => variant("None", Vec::new()),
        CompactionPolicy::SummarizeWhen { max_context_tokens } => variant(
            "SummarizeWhen",
            vec![HostValue::UInt(*max_context_tokens as u128)],
        ),
    }
}

fn message_value(message: &SessionMessage) -> HostValue {
    record([
        ("id", HostValue::String(message.id.clone())),
        ("from", option(message.from.clone().map(HostValue::String))),
        ("to", option(message.to.clone().map(HostValue::String))),
        (
            "role",
            HostValue::String(message_role_name(message.role).to_owned()),
        ),
        ("session", HostValue::String(message.session.id.clone())),
        ("created_at", HostValue::String(message.created_at.clone())),
        ("payload", message.payload.clone()),
        ("provenance", option(message.provenance.clone())),
        (
            "dedup_key",
            option(message.dedup_key.clone().map(HostValue::String)),
        ),
    ])
}

fn message_role_name(role: SessionMessageRole) -> &'static str {
    match role {
        SessionMessageRole::System => "system",
        SessionMessageRole::User => "user",
        SessionMessageRole::Assistant => "assistant",
        SessionMessageRole::Tool => "tool",
    }
}
