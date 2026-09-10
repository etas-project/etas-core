use crate::{HostError, HostErrorCode};

const TOKEN_LEN: usize = 4 + 64 + 1 + 32 + 1 + 16;

#[derive(Clone, PartialEq, Eq)]
pub struct MemoryVersion {
    token: String,
}

impl std::fmt::Debug for MemoryVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MemoryVersion(<redacted>)")
    }
}

impl MemoryVersion {
    pub fn parse(token: &str) -> Result<Self, HostError> {
        if token.len() != TOKEN_LEN || !token.starts_with("mv1:") {
            return Err(invalid_token());
        }
        let parts: Vec<_> = token[4..].split(':').collect();
        if parts.len() != 3
            || parts[0].len() != 64
            || parts[1].len() != 32
            || parts[2].len() != 16
            || parts.iter().any(|part| {
                !part
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        {
            return Err(invalid_token());
        }
        let revision = u64::from_str_radix(parts[2], 16).map_err(|_| invalid_token())?;
        if revision == 0 || revision > i64::MAX as u64 {
            return Err(invalid_token());
        }
        Ok(Self {
            token: token.to_owned(),
        })
    }

    pub fn as_token(&self) -> &str {
        &self.token
    }

    pub(crate) fn issue(
        scope: &str,
        generation: &StoreGeneration,
        revision: i64,
    ) -> Result<Self, HostError> {
        Self::parse(&format!("mv1:{scope}:{}:{revision:016x}", generation.0))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct StoreGeneration(String);

impl StoreGeneration {
    pub(crate) fn new() -> Result<Self, HostError> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "cannot allocate storage generation",
            )
        })?;
        Ok(Self(format!("{:032x}", u128::from_be_bytes(bytes))))
    }

    pub(crate) fn from_stored(value: String) -> Result<Self, HostError> {
        if value.len() != 32
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(HostError::new(
                HostErrorCode::SchemaMismatch,
                "invalid stored generation",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn scope_identity(namespace: &str, region: &str, path: &[String]) -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(b"etas.storage.scope.v1");
    for part in std::iter::once(namespace)
        .chain(std::iter::once(region))
        .chain(path.iter().map(String::as_str))
    {
        hash.update(&(part.len() as u64).to_le_bytes());
        hash.update(part.as_bytes());
    }
    hash.finalize().to_hex().to_string()
}

pub(crate) fn next_revision(revision: i64) -> Result<i64, HostError> {
    if revision < 0 {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "negative stored revision",
        ));
    }
    revision.checked_add(1).ok_or_else(|| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "storage revision space exhausted",
        )
    })
}

fn invalid_token() -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "invalid or obsolete memory version token",
    )
}
