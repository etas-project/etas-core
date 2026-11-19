use crate::{HostError, HostErrorCode};

/// Operating-system entropy only. Source code owns operation-key spelling,
/// expiration policy and fingerprints.
pub fn fresh_nonce() -> Result<u128, HostError> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "operating-system entropy is unavailable",
        )
    })?;
    Ok(u128::from_be_bytes(bytes))
}
