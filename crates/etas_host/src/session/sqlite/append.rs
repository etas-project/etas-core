use super::*;
impl SessionDatabase<'_> {
    pub(super) fn resolve(&mut self, config: SessionConfig) -> Result<SessionResult, HostError> {
        let connection = &mut *self.connection;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sqlite_error)?;
        let result = apply_resolve(&tx, config, self.limits)?;
        if let Some(operation) = self.operation {
            operation.check()?;
        }
        tx.commit().map_err(sqlite_error)?;
        Ok(result)
    }

    pub(super) fn append(&mut self, message: SessionMessage) -> Result<SessionResult, HostError> {
        let transaction = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(sqlite_error)?;
        let (result, _) = apply_append(&transaction, message, self.limits)?;
        if let Some(operation) = self.operation {
            operation.check()?;
        }
        transaction.commit().map_err(sqlite_error)?;
        Ok(result)
    }
}

pub(super) fn apply_resolve(
    connection: &Connection,
    config: SessionConfig,
    limits: &crate::StorageLimits,
) -> Result<SessionResult, HostError> {
    if config.id.is_empty() {
        return Err(invalid_request("session id must not be empty"));
    }
    let config_json = encode_config(&config, limits)?;
    let created = connection
        .execute(
            "INSERT INTO sessions (id, config_json, summary_text, summary_message_count)
                 VALUES (?1, ?2, NULL, NULL)
                 ON CONFLICT(id) DO NOTHING",
            params![config.id, config_json],
        )
        .map_err(sqlite_error)?
        == 1;
    if !created {
        let stored: String = connection
            .query_row(
                "SELECT config_json FROM sessions WHERE id=?1",
                [&config.id],
                |row| bounded_text(row, 0, limits.max_value_bytes),
            )
            .map_err(sqlite_error)?;
        if stored != config_json {
            return Err(invalid_request(
                "session identity already has a different configuration",
            ));
        }
    } else {
        connection
            .execute(
                "INSERT INTO session_sequences(session_id,last_ordinal) VALUES(?1,-1)",
                [&config.id],
            )
            .map_err(sqlite_error)?;
    }
    Ok(SessionResult::Resolved {
        session: SessionRef { id: config.id },
        created,
    })
}

pub(super) fn apply_append(
    connection: &Connection,
    message: SessionMessage,
    limits: &crate::StorageLimits,
) -> Result<(SessionResult, i64), HostError> {
    if message.id.is_empty() || message.session.id.is_empty() {
        return Err(invalid_request(
            "message and session identities must not be empty",
        ));
    }
    let payload_json = crate::value::tagged::encode_with_limits(&message.payload, limits)?;
    let provenance_json = message
        .provenance
        .as_ref()
        .map(|value| crate::value::tagged::encode_with_limits(value, limits))
        .transpose()?;
    if !session_exists(connection, &message.session.id)? {
        return Err(invalid_request(
            "cannot append message to an unresolved session",
        ));
    }
    if let Some(dedup_key) = &message.dedup_key
        && let Some(existing) =
            select_message_by_dedup(connection, &message.session.id, dedup_key, limits)?
    {
        crate::session::dedup::validate_replay(&existing, &message)?;
        let ordinal = connection
            .query_row(
                "SELECT ordinal FROM session_messages WHERE session_id=?1 AND message_id=?2",
                params![existing.session.id, existing.id],
                |row| row.get(0),
            )
            .map_err(sqlite_error)?;
        return Ok((
            SessionResult::Appended {
                message: existing,
                deduplicated: true,
            },
            ordinal,
        ));
    }
    if select_message(connection, &message.session.id, &message.id, limits)?.is_some() {
        return Err(invalid_request("session message id already exists"));
    }
    let ordinal = next_ordinal(connection, &message.session.id)?;
    connection.execute("INSERT INTO session_messages
        (session_id,message_id,ordinal,role,from_participant,to_participant,created_at,payload_json,provenance_json,dedup_key)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![message.session.id,message.id,ordinal,role_name(message.role),
        message.from,message.to,message.created_at,payload_json,provenance_json,message.dedup_key]).map_err(sqlite_error)?;
    Ok((
        SessionResult::Appended {
            message,
            deduplicated: false,
        },
        ordinal,
    ))
}

pub(super) fn next_ordinal(connection: &Connection, session: &str) -> Result<i64, HostError> {
    let previous: i64 = connection
        .query_row(
            "SELECT last_ordinal FROM session_sequences WHERE session_id=?1",
            [session],
            |row| row.get(0),
        )
        .map_err(sqlite_error)?;
    if previous < -1 {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "invalid session ordinal",
        ));
    }
    let next = previous.checked_add(1).ok_or_else(|| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "session ordinal space exhausted",
        )
    })?;
    let changed = connection
        .execute(
            "UPDATE session_sequences SET last_ordinal=?1 WHERE session_id=?2 AND last_ordinal=?3",
            params![next, session, previous],
        )
        .map_err(sqlite_error)?;
    if changed != 1 {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "session ordinal changed within write transaction",
        ));
    }
    Ok(next)
}

#[cfg(test)]
mod tests;
