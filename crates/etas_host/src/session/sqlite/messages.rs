use super::*;
#[cfg(test)]
mod tests;
pub(super) fn select_message(
    connection: &Connection,
    session: &str,
    message: &str,
    limits: &crate::StorageLimits,
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
        .query_row(params![session, message], |row| {
            Ok(read_message(row, session, limits, limits.max_result_bytes))
        })
        .optional()
        .map_err(sqlite_error)?
        .transpose()
}

pub(super) fn select_message_by_dedup(
    connection: &Connection,
    session: &str,
    dedup_key: &str,
    limits: &crate::StorageLimits,
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
        .query_row(params![session, dedup_key], |row| {
            Ok(read_message(row, session, limits, limits.max_result_bytes))
        })
        .optional()
        .map_err(sqlite_error)?
        .transpose()
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

fn stored_message_from_row(
    row: &rusqlite::Row<'_>,
    limits: &crate::StorageLimits,
    remaining_bytes: usize,
) -> rusqlite::Result<StoredSessionMessage> {
    let mut bytes = 0usize;
    for index in 0..8 {
        if let rusqlite::types::ValueRef::Text(value) = row.get_ref(index)? {
            bytes = bytes
                .checked_add(value.len())
                .ok_or(rusqlite::Error::InvalidQuery)?;
        }
    }
    // Inspect SQLite's borrowed cells before allocating strings. This also
    // bounds transient wire storage, not just the eventual decoded message.
    if bytes > limits.max_value_bytes || bytes > remaining_bytes {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(HostError::new(
                HostErrorCode::BudgetExceeded,
                "stored session message exceeds byte limit",
            )),
        ));
    }
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

pub(super) fn read_message(
    row: &rusqlite::Row<'_>,
    session: &str,
    limits: &crate::StorageLimits,
    remaining_bytes: usize,
) -> Result<SessionMessage, HostError> {
    let message = stored_message_from_row(row, limits, remaining_bytes).map_err(sqlite_error)?;
    let decode_limits = crate::StorageLimits {
        max_value_bytes: limits.max_value_bytes.min(remaining_bytes),
        ..limits.clone()
    };
    let message = decode_message(session, message, &decode_limits)?;
    if message.storage_size(&decode_limits)? > remaining_bytes {
        return Err(HostError::new(
            HostErrorCode::BudgetExceeded,
            "decoded session message exceeds remaining result budget",
        ));
    }
    Ok(message)
}

fn decode_message(
    session: &str,
    message: StoredSessionMessage,
    limits: &crate::StorageLimits,
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
        payload: crate::value::tagged::decode_with_limits(&message.payload_json, limits)?,
        provenance: message
            .provenance_json
            .as_deref()
            .map(|value| crate::value::tagged::decode_with_limits(value, limits))
            .transpose()?,
        dedup_key: message.dedup_key,
    })
}
