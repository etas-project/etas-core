use crate::{
    ContextPolicy, HostTraceFieldSensitivity, HostTracePayload, HostValue, MemoryOperation,
    MemoryQuery, MemoryRequest, MemoryVersion, RetentionPolicy, SessionConfig, SessionMessage,
    SessionMessageRole, SessionOperation, SessionRequest, WriteCondition,
};

use super::{HostTraceRequest, option, record, strings, variant};

impl HostTraceRequest for crate::session::SessionContextPublication {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("session", "Session.publish_context")
            .with_field(
                "session",
                HostValue::String(self.session.id.clone()),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "fence",
                HostValue::String(self.fence.as_token().to_owned()),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "content",
                record([
                    ("text", HostValue::String(self.content.text.clone())),
                    (
                        "provenance",
                        HostValue::Record(
                            self.content
                                .provenance
                                .iter()
                                .map(|(k, v)| (k.clone(), HostValue::String(v.clone())))
                                .collect(),
                        ),
                    ),
                ]),
                HostTraceFieldSensitivity::Sensitive,
            )
            .with_field(
                "operation",
                record([
                    (
                        "key",
                        HostValue::String(self.operation.key.as_str().to_owned()),
                    ),
                    (
                        "fingerprint",
                        HostValue::String(self.operation.request_fingerprint.clone()),
                    ),
                ]),
                HostTraceFieldSensitivity::Sensitive,
            )
    }
}

impl HostTraceRequest for MemoryRequest {
    fn trace_payload(&self) -> HostTracePayload {
        let (action, operation) = match &self.operation {
            MemoryOperation::Get { key } => ("Memory.read", variant("Get", vec![key.clone()])),
            MemoryOperation::Put {
                key,
                value,
                condition,
            } => (
                "Memory.write",
                variant(
                    "Put",
                    vec![key.clone(), value.clone(), condition_value(condition)],
                ),
            ),
            MemoryOperation::Delete { key, condition } => (
                "Memory.write",
                variant("Delete", vec![key.clone(), condition_value(condition)]),
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
        memory_payload(&self.store, action, operation)
    }
}

impl HostTraceRequest for crate::memory::MemoryWriteRequest {
    fn trace_payload(&self) -> HostTracePayload {
        use crate::memory::{MemoryMutation, MemoryWriteOperation};
        let (action, operation) = match &self.operation {
            MemoryWriteOperation::Mutate { key, mutation } => {
                let value = match mutation {
                    MemoryMutation::Put {
                        key,
                        value,
                        condition,
                    } => variant(
                        "Put",
                        vec![key.clone(), value.clone(), condition_value(condition)],
                    ),
                    MemoryMutation::Delete { key, condition } => {
                        variant("Delete", vec![key.clone(), condition_value(condition)])
                    }
                };
                (
                    "Memory.write",
                    variant(
                        "Commit",
                        vec![HostValue::String(key.as_str().into()), value],
                    ),
                )
            }
            MemoryWriteOperation::Reconcile { operation } => (
                "Memory.read",
                variant(
                    "Reconcile",
                    vec![
                        HostValue::String(operation.key.as_str().into()),
                        HostValue::String(operation.request_fingerprint.clone()),
                    ],
                ),
            ),
        };
        memory_payload(&self.store, action, operation)
    }
}

fn memory_payload(
    store: &crate::StoreRef,
    action: &'static str,
    operation: HostValue,
) -> HostTracePayload {
    HostTracePayload::new("memory", action)
        .with_field(
            "store",
            record([
                ("region", HostValue::String(store.region.stable_id.clone())),
                (
                    "schema_fingerprint",
                    option(
                        store
                            .region
                            .schema_fingerprint
                            .clone()
                            .map(HostValue::String),
                    ),
                ),
                ("path", strings(&store.path)),
            ]),
            HostTraceFieldSensitivity::Sensitive,
        )
        .with_field("operation", operation, HostTraceFieldSensitivity::Sensitive)
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
        };
        HostTracePayload::new("session", action).with_field(
            "operation",
            operation,
            HostTraceFieldSensitivity::Sensitive,
        )
    }
}

fn version(value: &MemoryVersion) -> HostValue {
    HostValue::String(value.as_token().to_owned())
}

fn condition_value(condition: &WriteCondition) -> HostValue {
    match condition {
        WriteCondition::Any => variant("Any", vec![]),
        WriteCondition::Missing => variant("Missing", vec![]),
        WriteCondition::Exists => variant("Exists", vec![]),
        WriteCondition::Match(value) => variant("Match", vec![version(value)]),
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
