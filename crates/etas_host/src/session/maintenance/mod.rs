mod intent;
mod protocol;
mod trace;
pub use intent::SessionRetentionIntent;
pub use protocol::*;

pub(super) fn cutoff(
    selected: Option<i128>,
    retention: &crate::RetentionPolicy,
) -> Result<Option<i128>, crate::HostError> {
    let current = match retention {
        crate::RetentionPolicy::Forever => None,
        crate::RetentionPolicy::Days(days) => Some(
            super::retention::current_unix_seconds()?.saturating_sub(i128::from(*days) * 86_400),
        ),
    };
    match (selected, current) {
        (None, _) => Ok(None),
        (Some(selected), Some(current)) if selected <= current => Ok(Some(selected)),
        _ => Err(invalid(
            "retention selection exceeds current configured policy",
        )),
    }
}

pub(super) fn expired(timestamp: &str, cutoff: Option<i128>) -> Result<bool, crate::HostError> {
    match cutoff {
        None => Ok(false),
        Some(cutoff) => super::retention::parse_session_timestamp(timestamp)
            .map(|at| at < cutoff)
            .ok_or_else(|| invalid("invalid session timestamp for retention maintenance")),
    }
}

pub(super) fn invalid(message: &str) -> crate::HostError {
    crate::HostError::new(crate::HostErrorCode::InvalidRequest, message)
}

pub(super) fn rejected(
    operation: crate::StorageOperationRef,
    error: crate::HostError,
) -> SessionRetentionOutcome {
    crate::WriteOutcome::NotCommitted {
        operation,
        reason: SessionRetentionRejection::Rejected(error),
    }
}
