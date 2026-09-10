use super::*;
use crate::session::paging::{HistoryCursor, context_count, limit_error, page_size};

pub(super) fn load(
    db: &mut SessionDatabase<'_>,
    session: SessionRef,
    context: ContextPolicy,
    cursor: Option<SessionCursor>,
    limit: Option<u32>,
) -> Result<SessionResult, HostError> {
    let limit = page_size(limit, db.limits)?;
    let transaction = db.connection.transaction().map_err(sqlite_error)?;
    let fence_state = super::fence::state(&transaction, &session.id)?;
    let published_context = super::context::load(
        &transaction,
        &session.id,
        fence_state.context_version,
        db.limits,
    )?;
    let include_summary = matches!(context, ContextPolicy::SummaryPlusRecent { .. });
    let (config, generation, upper, summary) = transaction.query_row(
        "SELECT config_json, generation, last_ordinal,
            CASE WHEN ?2 THEN summary_text ELSE NULL END,
            CASE WHEN ?2 THEN summary_message_count ELSE NULL END
         FROM sessions JOIN session_history_generations ON session_history_generations.session_id=sessions.id
         JOIN session_sequences ON session_sequences.session_id=sessions.id WHERE sessions.id=?1",
        params![session.id, include_summary], |row| {
            for index in [0,1,3] {
                if let rusqlite::types::ValueRef::Text(text) = row.get_ref(index)?
                    && text.len() > db.limits.max_value_bytes {
                    return Err(rusqlite::Error::FromSqlConversionFailure(index, rusqlite::types::Type::Text, Box::new(limit_error())));
                }
            }
            let summary = match (row.get::<_,Option<String>>(3)?, row.get::<_,Option<i64>>(4)?) {
                (None, None) => None,
                (Some(text), Some(count)) => Some((text, count)),
                _ => return Err(rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text,
                    Box::new(HostError::new(HostErrorCode::SchemaMismatch, "incomplete session summary metadata")))),
            };
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,i64>(2)?, summary))
        }).optional().map_err(sqlite_error)?.ok_or_else(|| invalid_request("session history identity is missing"))?;
    if upper < -1
        || generation.len() != 32
        || !generation.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(invalid_request("invalid session history identity"));
    }
    let config: Value = serde_json::from_str(&config).map_err(json_error)?;
    let retention = retention_policy_from_json(
        config
            .get("retention")
            .ok_or_else(|| invalid_request("missing session retention"))?,
    )?;
    let mut position = match cursor {
        Some(cursor) => HistoryCursor::decode(
            &cursor,
            &session.id,
            &context,
            &generation,
            &fence_state.key,
            upper,
            db.limits,
        )?,
        None => {
            let mut cursor = HistoryCursor::new(&session.id, &context, upper, &retention)?;
            if let Some(count) = context_count(&context) {
                cursor.lower = context_lower(
                    &transaction,
                    &session.id,
                    &cursor,
                    count,
                    db.limits,
                    db.operation,
                )?;
                cursor.after = cursor.lower - 1;
            }
            cursor
        }
    };
    let summary = summary
        .filter(|_| published_context.is_none())
        .map(|(text, count)| {
            Ok(SessionSummary {
                text,
                message_count: usize::try_from(count).map_err(count_error)?,
            })
        })
        .transpose()?;
    let fence = crate::session::SessionHistoryFence::issue(&position, &fence_state, db.limits)?;
    let mut bytes = summary
        .as_ref()
        .map_or(0, |summary| summary.text.len())
        .checked_add(fence.as_token().len())
        .ok_or_else(limit_error)?;
    if let Some(context) = &published_context {
        bytes = bytes
            .checked_add(context.storage_size(db.limits)?)
            .ok_or_else(limit_error)?;
    }
    if bytes > db.limits.max_result_bytes {
        return Err(limit_error());
    }
    let mut statement = transaction
        .prepare(
            "SELECT message_id, role, from_participant, to_participant, created_at,
            payload_json, provenance_json, dedup_key, ordinal FROM session_messages
         WHERE session_id=?1 AND ordinal>?2 AND ordinal>=?3 AND ordinal<=?4
         ORDER BY ordinal LIMIT ?5",
        )
        .map_err(sqlite_error)?;
    let mut rows = statement
        .query(params![
            session.id,
            position.after,
            position.lower,
            position.upper,
            work_limit(db.limits)?
        ])
        .map_err(sqlite_error)?;
    let mut messages = Vec::new();
    let mut visited = 0;
    let mut more = false;
    while let Some(row) = rows.next().map_err(sqlite_error)? {
        check_work(db.operation, &mut visited, db.limits)?;
        if !retains_row(row, 4, &position, db.limits)? {
            continue;
        }
        if messages.len() == limit {
            more = true;
            break;
        }
        let message = read_message(
            row,
            &session.id,
            db.limits,
            db.limits
                .max_result_bytes
                .checked_sub(bytes)
                .ok_or_else(limit_error)?,
        )?;
        bytes = bytes
            .checked_add(message.storage_size(db.limits)?)
            .ok_or_else(limit_error)?;
        if bytes > db.limits.max_result_bytes {
            return Err(limit_error());
        }
        position.after = row.get(8).map_err(sqlite_error)?;
        messages.push(message);
    }
    let cursor = more
        .then(|| position.encode(&generation, &fence_state.key, db.limits))
        .transpose()?;
    if bytes
        .checked_add(cursor.as_ref().map_or(0, |cursor| cursor.opaque.len()))
        .ok_or_else(limit_error)?
        > db.limits.max_result_bytes
    {
        return Err(limit_error());
    }
    Ok(SessionResult::History {
        session,
        fence,
        published_context,
        messages,
        summary,
        cursor,
    })
}

fn context_lower(
    connection: &Connection,
    session: &str,
    cursor: &HistoryCursor,
    count: usize,
    limits: &crate::StorageLimits,
    operation: Option<&SqliteOperation>,
) -> Result<i64, HostError> {
    if count == 0 {
        return cursor.upper.checked_add(1).ok_or_else(limit_error);
    }
    let mut statement = connection.prepare("SELECT ordinal,created_at FROM session_messages WHERE session_id=?1 AND ordinal<=?2 ORDER BY ordinal DESC LIMIT ?3").map_err(sqlite_error)?;
    let mut rows = statement
        .query(params![session, cursor.upper, work_limit(limits)?])
        .map_err(sqlite_error)?;
    let mut selected = 0;
    let mut visited = 0;
    while let Some(row) = rows.next().map_err(sqlite_error)? {
        check_work(operation, &mut visited, limits)?;
        if cursor.retained_after.is_some() && !retains_row(row, 1, cursor, limits)? {
            continue;
        }
        selected += 1;
        if selected == count {
            return row.get(0).map_err(sqlite_error);
        }
    }
    Ok(0)
}

fn retains_row(
    row: &rusqlite::Row<'_>,
    column: usize,
    cursor: &HistoryCursor,
    limits: &crate::StorageLimits,
) -> Result<bool, HostError> {
    let raw = row.get_ref(column).map_err(sqlite_error)?;
    let text = raw
        .as_str()
        .map_err(|_| invalid_request("session timestamp is not text"))?;
    if text.len() > limits.max_value_bytes {
        return Err(limit_error());
    }
    cursor.retains(text)
}
fn work_limit(limits: &crate::StorageLimits) -> Result<i64, HostError> {
    i64::try_from(limits.max_scan_rows)
        .ok()
        .and_then(|n| n.checked_add(1))
        .ok_or_else(limit_error)
}
fn check_work(
    operation: Option<&SqliteOperation>,
    visited: &mut usize,
    limits: &crate::StorageLimits,
) -> Result<(), HostError> {
    if let Some(operation) = operation {
        operation.check()?;
    }
    if *visited >= limits.max_scan_rows {
        return Err(limit_error());
    }
    *visited += 1;
    Ok(())
}
