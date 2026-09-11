use super::*;
pub(super) fn bounded_text(
    row: &rusqlite::Row<'_>,
    column: usize,
    maximum: usize,
) -> rusqlite::Result<String> {
    let value = row.get_ref(column)?;
    let text = value.as_str().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, value.data_type(), Box::new(error))
    })?;
    if text.len() > maximum {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            column,
            value.data_type(),
            Box::new(HostError::new(
                HostErrorCode::BudgetExceeded,
                "stored session metadata exceeds configured byte limit",
            )),
        ));
    }
    Ok(text.to_owned())
}

pub(super) fn encode_config(
    config: &SessionConfig,
    limits: &crate::StorageLimits,
) -> Result<String, HostError> {
    if config.id.len() > limits.max_value_bytes {
        return Err(crate::session::write::limit_error());
    }
    let value = session_config_json(config)?;
    struct Counter {
        used: usize,
        maximum: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let used = self
                .used
                .checked_add(bytes.len())
                .filter(|used| *used <= self.maximum)
                .ok_or_else(|| std::io::Error::other("session config byte limit exceeded"))?;
            self.used = used;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    // Preflight escaped JSON size without constructing an oversized encoded String.
    serde_json::to_writer(
        Counter {
            used: 0,
            maximum: limits.max_value_bytes,
        },
        &value,
    )
    .map_err(|_| crate::session::write::limit_error())?;
    serde_json::to_string(&value).map_err(json_error)
}

pub(super) fn session_config_json(config: &SessionConfig) -> Result<Value, HostError> {
    Ok(json!({
        "id": config.id,
        "context": context_policy_json(&config.context),
        "retention": retention_policy_json(&config.retention),
    }))
}

pub(super) fn context_policy_json(policy: &ContextPolicy) -> Value {
    match policy {
        ContextPolicy::All => json!({ "kind": "All" }),
        ContextPolicy::LastTurns(turns) => json!({ "kind": "LastTurns", "turns": turns }),
        ContextPolicy::SummaryPlusRecent { recent } => {
            json!({ "kind": "SummaryPlusRecent", "recent": recent })
        }
    }
}

pub(super) fn retention_policy_json(policy: &RetentionPolicy) -> Value {
    match policy {
        RetentionPolicy::Forever => json!({ "kind": "Forever" }),
        RetentionPolicy::Days(days) => json!({ "kind": "Days", "days": days }),
    }
}

pub(super) fn retention_policy_from_json(value: &Value) -> Result<RetentionPolicy, HostError> {
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

pub(super) fn role_name(role: SessionMessageRole) -> &'static str {
    match role {
        SessionMessageRole::System => "system",
        SessionMessageRole::User => "user",
        SessionMessageRole::Assistant => "assistant",
        SessionMessageRole::Tool => "tool",
    }
}

pub(super) fn role_from_name(value: &str) -> Result<SessionMessageRole, String> {
    match value {
        "system" => Ok(SessionMessageRole::System),
        "user" => Ok(SessionMessageRole::User),
        "assistant" => Ok(SessionMessageRole::Assistant),
        "tool" => Ok(SessionMessageRole::Tool),
        other => Err(format!("unknown session message role `{other}`")),
    }
}
