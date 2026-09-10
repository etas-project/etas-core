use crate::{HostError, HostErrorCode};

/// The expiry is part of identity, so evicting evidence never permits this key to execute again.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(transparent)]
pub struct StorageOperationKey {
    token: String,
    #[serde(skip)]
    expires_at: u64,
}

impl StorageOperationKey {
    pub fn new(retention: std::time::Duration) -> Result<Self, HostError> {
        let expires = now()?
            .checked_add(retention.as_secs())
            .ok_or_else(invalid)?;
        if retention.as_secs() == 0 || expires > i64::MAX as u64 {
            return Err(invalid());
        }
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "cannot allocate storage operation identity",
            )
        })?;
        Self::parse(&format!(
            "so1:{expires:016x}:{:032x}",
            u128::from_be_bytes(nonce)
        ))
    }

    pub fn parse(value: &str) -> Result<Self, HostError> {
        if value.len() != 53
            || !value.starts_with("so1:")
            || value.as_bytes()[20] != b':'
            || value[4..20]
                .bytes()
                .chain(value[21..].bytes())
                .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
        {
            return Err(invalid());
        }
        let expires_at = u64::from_str_radix(&value[4..20], 16).map_err(|_| invalid())?;
        if expires_at == 0 || expires_at > i64::MAX as u64 {
            return Err(invalid());
        }
        Ok(Self {
            token: value.to_owned(),
            expires_at,
        })
    }
    pub fn as_str(&self) -> &str {
        &self.token
    }
    pub fn derive(&self, occurrence: u64) -> Result<Self, HostError> {
        let mut hash = blake3::Hasher::new_derive_key("etas.storage.occurrence.v1");
        hash.update(self.token.as_bytes());
        hash.update(&occurrence.to_be_bytes());
        let nonce = hash.finalize();
        Self::parse(&format!(
            "so1:{:016x}:{}",
            self.expires_at,
            &nonce.to_hex()[..32]
        ))
    }
    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
    pub fn is_expired(&self) -> Result<bool, HostError> {
        Ok(self.expires_at() <= now()?)
    }
    pub(crate) fn validate_window(&self, maximum_seconds: u64) -> Result<(), HostError> {
        let now = now()?;
        if self.expires_at() <= now || self.expires_at() - now > maximum_seconds {
            return Err(invalid());
        }
        Ok(())
    }
}

impl<'de> serde::Deserialize<'de> for StorageOperationKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let token = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(&token).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageOperationRef {
    pub key: StorageOperationKey,
    pub request_fingerprint: String,
}
impl StorageOperationRef {
    pub fn validate(&self) -> Result<(), HostError> {
        if self.request_fingerprint.len() != 64
            || self
                .request_fingerprint
                .bytes()
                .any(|b| !b.is_ascii_digit() && !(b'a'..=b'f').contains(&b))
        {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "invalid storage request fingerprint",
            ));
        }
        Ok(())
    }
}

pub(crate) fn now() -> Result<u64, HostError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "storage clock precedes epoch",
            )
        })
}
fn invalid() -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "invalid or expired storage operation key",
    )
}
