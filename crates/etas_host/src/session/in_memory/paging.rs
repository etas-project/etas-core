use super::*;
use crate::session::paging::{HistoryCursor, context_count, limit_error, page_size};

pub(super) fn load(
    state: &SessionState,
    session: SessionRef,
    context: ContextPolicy,
    cursor: Option<SessionCursor>,
    limit: Option<u32>,
    limits: &crate::StorageLimits,
) -> Result<SessionResult, HostError> {
    let size = page_size(limit, limits)?;
    let upper = state.last_ordinal;
    let mut position = match cursor {
        Some(cursor) => HistoryCursor::decode(
            &cursor,
            &session.id,
            &context,
            state.generation.as_str(),
            &state.history_key,
            upper,
            limits,
        )?,
        None => {
            let mut cursor =
                HistoryCursor::new(&session.id, &context, upper, &state.config.retention)?;
            if let Some(count) = context_count(&context) {
                if count == 0 {
                    cursor.lower = upper + 1;
                } else {
                    let mut selected = 0;
                    for (visited, (index, message)) in state.messages.iter().rev().enumerate() {
                        if visited >= limits.max_scan_rows {
                            return Err(limit_error());
                        }
                        if cursor.retains(&message.created_at)? {
                            selected += 1;
                            if selected == count {
                                cursor.lower = *index;
                                break;
                            }
                        }
                    }
                }
                cursor.after = cursor.lower - 1;
            }
            cursor
        }
    };
    let summary = match context {
        ContextPolicy::SummaryPlusRecent { .. } if state.published_context.is_none() => {
            state.summary.as_ref()
        }
        _ => None,
    };
    let fence =
        crate::session::SessionHistoryFence::issue(&position, &history_state(state)?, limits)?;
    let mut bytes = summary
        .map_or(0, |summary| summary.text.len())
        .checked_add(fence.as_token().len())
        .ok_or_else(limit_error)?;
    if let Some(context) = &state.published_context {
        bytes = bytes
            .checked_add(context.storage_size(limits)?)
            .ok_or_else(limit_error)?;
    }
    if bytes > limits.max_result_bytes
        || summary.is_some_and(|summary| summary.text.len() > limits.max_value_bytes)
    {
        return Err(limit_error());
    }
    let mut messages = Vec::new();
    let mut more = false;
    let start = position.after + 1;
    let end = position.upper + 1;
    for (visited, (ordinal, message)) in state.messages.range(start..end).enumerate() {
        if visited >= limits.max_scan_rows {
            return Err(limit_error());
        }
        if !position.retains(&message.created_at)? {
            continue;
        }
        if messages.len() == size {
            more = true;
            break;
        }
        bytes = bytes
            .checked_add(message.storage_size(limits)?)
            .ok_or_else(limit_error)?;
        if bytes > limits.max_result_bytes {
            return Err(limit_error());
        }
        messages.push(message.clone());
        position.after = *ordinal;
    }
    let cursor = more
        .then(|| position.encode(state.generation.as_str(), &state.history_key, limits))
        .transpose()?;
    if bytes
        .checked_add(cursor.as_ref().map_or(0, |cursor| cursor.opaque.len()))
        .ok_or_else(limit_error)?
        > limits.max_result_bytes
    {
        return Err(limit_error());
    }
    Ok(SessionResult::History {
        session,
        fence,
        published_context: state.published_context.clone(),
        messages,
        summary: summary.cloned(),
        cursor,
    })
}

pub(super) fn history_state(
    state: &SessionState,
) -> Result<crate::session::context::fence::HistoryState, HostError> {
    Ok(crate::session::context::fence::HistoryState {
        incarnation: state.storage_generation.as_str().to_owned(),
        revision: state.generation.as_str().to_owned(),
        upper: state.last_ordinal,
        context_version: state.context_version,
        key: state.history_key.clone(),
    })
}
