use crate::{
    ContextPolicy, HostError, HostErrorCode, RetentionPolicy, SessionCursor, StorageLimits,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HistoryCursor {
    version: u8,
    session: String,
    context: String,
    pub upper: i64,
    pub lower: i64,
    pub after: i64,
    pub retained_after: Option<i128>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    cursor: HistoryCursor,
    seal: String,
}

impl HistoryCursor {
    pub(super) fn session_id(&self) -> &str {
        &self.session
    }
    pub fn new(
        session: &str,
        context: &ContextPolicy,
        upper: i64,
        retention: &RetentionPolicy,
    ) -> Result<Self, HostError> {
        let retained_after = match retention {
            RetentionPolicy::Forever => None,
            RetentionPolicy::Days(days) => Some(
                super::retention::current_unix_seconds()?
                    .saturating_sub(i128::from(*days) * 86_400),
            ),
        };
        Ok(Self {
            version: 2,
            session: session.to_owned(),
            context: context_key(context),
            upper,
            lower: 0,
            after: -1,
            retained_after,
        })
    }

    pub fn decode(
        token: &SessionCursor,
        session: &str,
        context: &ContextPolicy,
        generation: &str,
        key: &super::context::fence::HistoryKey,
        upper: i64,
        limits: &StorageLimits,
    ) -> Result<Self, HostError> {
        if token.opaque.len() > limits.max_value_bytes {
            return Err(invalid_cursor());
        }
        let envelope: Envelope =
            serde_json::from_str(&token.opaque).map_err(|_| invalid_cursor())?;
        let cursor = envelope.cursor;
        let expected = cursor.seal(generation, key)?;
        if envelope.seal != expected
            || !cursor.valid_shape()
            || cursor.session != session
            || cursor.context != context_key(context)
            || cursor.upper > upper
        {
            return Err(invalid_cursor());
        }
        Ok(cursor)
    }

    pub fn encode(
        &self,
        generation: &str,
        key: &super::context::fence::HistoryKey,
        limits: &StorageLimits,
    ) -> Result<SessionCursor, HostError> {
        let opaque = serde_json::to_string(&Envelope {
            cursor: self.clone(),
            seal: self.seal(generation, key)?,
        })
        .map_err(|_| invalid_cursor())?;
        if opaque.len() > limits.max_value_bytes {
            return Err(limit_error());
        }
        Ok(SessionCursor { opaque })
    }

    fn seal(
        &self,
        generation: &str,
        key: &super::context::fence::HistoryKey,
    ) -> Result<String, HostError> {
        key.seal("etas.session.history.cursor.v2", &(generation, self))
    }

    pub(super) fn valid_shape(&self) -> bool {
        self.version == 2
            && !self.session.is_empty()
            && !self.context.is_empty()
            && self.upper >= -1
            && self.upper < i64::MAX
            && self.lower >= 0
            && self.after >= -1
            && self.after <= self.upper
            && self.lower - 1 <= self.upper
            && self.after >= self.lower - 1
    }

    pub fn retains(&self, created_at: &str) -> Result<bool, HostError> {
        match self.retained_after {
            None => Ok(true),
            Some(cutoff) => super::retention::parse_session_timestamp(created_at)
                .map(|timestamp| timestamp >= cutoff)
                .ok_or_else(|| {
                    HostError::new(
                        HostErrorCode::InvalidRequest,
                        "invalid session timestamp for retention",
                    )
                }),
        }
    }
}

pub(super) fn page_size(limit: Option<u32>, limits: &StorageLimits) -> Result<usize, HostError> {
    let limit = limit
        .map(|n| n as usize)
        .unwrap_or(100.min(limits.max_page_entries));
    if limit == 0 || limit > limits.max_page_entries {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "session page limit is outside configured bounds",
        ));
    }
    Ok(limit)
}
pub(super) fn context_count(context: &ContextPolicy) -> Option<usize> {
    match context {
        ContextPolicy::All => None,
        ContextPolicy::LastTurns(n) | ContextPolicy::SummaryPlusRecent { recent: n } => {
            Some(n.saturating_mul(2))
        }
    }
}
fn context_key(context: &ContextPolicy) -> String {
    match context {
        ContextPolicy::All => "all".to_owned(),
        ContextPolicy::LastTurns(n) => format!("last:{n}"),
        ContextPolicy::SummaryPlusRecent { recent } => format!("summary:{recent}"),
    }
}
pub(super) fn limit_error() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "session history exceeds configured page/work limits",
    )
}
fn invalid_cursor() -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "invalid, foreign or invalidated session history cursor",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_generation_is_not_a_cursor_signing_key() {
        let limits = StorageLimits::default();
        let key = super::super::context::fence::HistoryKey::new().unwrap();
        let other = super::super::context::fence::HistoryKey::new().unwrap();
        let cursor =
            HistoryCursor::new("s", &ContextPolicy::All, 9, &RetentionPolicy::Forever).unwrap();
        let token = cursor
            .encode("published-generation", &key, &limits)
            .unwrap();
        assert!(
            HistoryCursor::decode(
                &token,
                "s",
                &ContextPolicy::All,
                "published-generation",
                &other,
                9,
                &limits
            )
            .is_err()
        );
        let legacy_key =
            blake3::derive_key("etas.session.history.cursor.v1", b"published-generation");
        let seal = blake3::keyed_hash(&legacy_key, &serde_json::to_vec(&cursor).unwrap())
            .to_hex()
            .to_string();
        let forged = SessionCursor {
            opaque: serde_json::to_string(&Envelope { cursor, seal }).unwrap(),
        };
        assert!(
            HistoryCursor::decode(
                &forged,
                "s",
                &ContextPolicy::All,
                "published-generation",
                &key,
                9,
                &limits
            )
            .is_err()
        );
    }

    #[test]
    fn cursor_preserves_original_retention_cutoff() {
        let limits = StorageLimits::default();
        let key = super::super::context::fence::HistoryKey::new().unwrap();
        let mut cursor =
            HistoryCursor::new("s", &ContextPolicy::All, 9, &RetentionPolicy::Days(1)).unwrap();
        cursor.retained_after = Some(100);
        let token = cursor.encode("generation", &key, &limits).unwrap();
        let decoded = HistoryCursor::decode(
            &token,
            "s",
            &ContextPolicy::All,
            "generation",
            &key,
            20,
            &limits,
        )
        .unwrap();
        assert_eq!(decoded.retained_after, Some(100));
        assert!(!decoded.retains("99").unwrap());
        assert!(decoded.retains("100").unwrap());
        assert_eq!(decoded.upper, 9);
        assert!(
            HistoryCursor::decode(
                &token,
                "s",
                &ContextPolicy::All,
                "replacement",
                &key,
                20,
                &limits
            )
            .is_err()
        );
    }

    #[test]
    fn maximum_retention_duration_roundtrips() {
        let limits = StorageLimits::default();
        let key = super::super::context::fence::HistoryKey::new().unwrap();
        let cursor = HistoryCursor::new(
            "s",
            &ContextPolicy::All,
            9,
            &RetentionPolicy::Days(u64::MAX),
        )
        .unwrap();
        let token = cursor.encode("generation", &key, &limits).unwrap();
        let decoded = HistoryCursor::decode(
            &token,
            "s",
            &ContextPolicy::All,
            "generation",
            &key,
            9,
            &limits,
        )
        .unwrap();
        assert_eq!(decoded.retained_after, cursor.retained_after);
    }
}
