use crate::session::paging::HistoryCursor;
use crate::{HostError, HostErrorCode, StorageLimits};
use serde::{Deserialize, Serialize};

/// Backend-issued selection evidence, not a storage authorization grant.
#[derive(Clone, PartialEq, Eq)]
pub struct SessionHistoryFence {
    token: String,
    session: String,
}

impl std::fmt::Debug for SessionHistoryFence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionHistoryFence([redacted])")
    }
}

impl SessionHistoryFence {
    pub(in crate::session) fn retention_selection(
        &self,
        state: &HistoryState,
        limits: &StorageLimits,
    ) -> Result<HistoryCursor, HostError> {
        let envelope = decode(&self.token, limits)?;
        if state
            .key
            .seal("etas.session.history.fence.v1", &envelope.claims)?
            != envelope.seal
            || envelope.claims.incarnation != state.incarnation
            || envelope.claims.selection.upper > state.upper
        {
            return Err(invalid());
        }
        // A retention sweep keeps its original cutoff/upper ordinal across its own
        // deletes. Ordinary history cursors still require the current revision.
        Ok(envelope.claims.selection)
    }

    pub(in crate::session) fn selection_upper(
        &self,
        limits: &StorageLimits,
    ) -> Result<i64, HostError> {
        Ok(decode(&self.token, limits)?.claims.selection.upper)
    }

    pub(in crate::session) fn is_current(
        &self,
        state: &HistoryState,
        limits: &StorageLimits,
    ) -> Result<bool, HostError> {
        let envelope = decode(&self.token, limits)?;
        let expected = state
            .key
            .seal("etas.session.history.fence.v1", &envelope.claims)?;
        if expected != envelope.seal {
            return Err(invalid());
        }
        let claims = envelope.claims;
        Ok(claims.incarnation == state.incarnation
            && claims.revision == state.revision
            && claims.selection.upper == state.upper
            && claims.context_version == state.context_version)
    }
    pub fn as_token(&self) -> &str {
        &self.token
    }

    pub fn session_id(&self) -> &str {
        &self.session
    }

    /// Decoding preserves evidence. Only the issuing backend can authenticate it.
    pub fn from_token(token: String, limits: &StorageLimits) -> Result<Self, HostError> {
        let envelope = decode(&token, limits)?;
        Ok(Self {
            session: envelope.claims.selection.session_id().to_owned(),
            token,
        })
    }

    pub(in crate::session) fn issue(
        cursor: &HistoryCursor,
        state: &HistoryState,
        limits: &StorageLimits,
    ) -> Result<Self, HostError> {
        if cursor.upper > state.upper {
            return Err(invalid());
        }
        let mut selection = cursor.clone();
        // All pages of a fixed selection carry the same publication evidence.
        selection.after = selection.lower - 1;
        let claims = Claims {
            format: 1,
            selection,
            incarnation: state.incarnation.clone(),
            revision: state.revision.clone(),
            context_version: state.context_version,
        };
        let seal = state.key.seal("etas.session.history.fence.v1", &claims)?;
        let token = serde_json::to_string(&Envelope { claims, seal }).map_err(|_| invalid())?;
        Self::from_token(token, limits)
    }
}

#[derive(Clone)]
pub(in crate::session) struct HistoryKey([u8; 32]);

impl std::fmt::Debug for HistoryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HistoryKey([redacted])")
    }
}

impl HistoryKey {
    pub fn new() -> Result<Self, HostError> {
        let mut key = [0; 32];
        getrandom::fill(&mut key).map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "cannot allocate session history key",
            )
        })?;
        Ok(Self(key))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HostError> {
        Ok(Self(bytes.try_into().map_err(|_| {
            HostError::new(HostErrorCode::SchemaMismatch, "invalid session history key")
        })?))
    }

    pub fn seal(&self, domain: &str, value: &impl Serialize) -> Result<String, HostError> {
        let bytes = serde_json::to_vec(value).map_err(|_| invalid())?;
        let key = blake3::derive_key(domain, &self.0);
        Ok(blake3::keyed_hash(&key, &bytes).to_hex().to_string())
    }
}

pub(in crate::session) struct HistoryState {
    pub incarnation: String,
    pub revision: String,
    pub upper: i64,
    pub context_version: u64,
    pub key: HistoryKey,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claims {
    format: u8,
    selection: HistoryCursor,
    incarnation: String,
    revision: String,
    context_version: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    claims: Claims,
    seal: String,
}

fn decode(token: &str, limits: &StorageLimits) -> Result<Envelope, HostError> {
    if token.len() > limits.max_value_bytes {
        return Err(crate::session::paging::limit_error());
    }
    let envelope: Envelope = serde_json::from_str(token).map_err(|_| invalid())?;
    let claims = &envelope.claims;
    if claims.format != 1
        || !hex(&claims.incarnation, 32)
        || !hex(&claims.revision, 32)
        || !hex(&envelope.seal, 64)
        || !claims.selection.valid_shape()
        || claims.selection.after != claims.selection.lower - 1
    {
        return Err(invalid());
    }
    Ok(envelope)
}

fn hex(text: &str, size: usize) -> bool {
    text.len() == size
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn invalid() -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "invalid session history fence",
    )
}
