use super::{ReplayKeyError, ReplayKeyErrorKind};
use crate::HostTracePayload;
use zeroize::Zeroizing;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReplayKeyId([u8; 32]);

#[derive(Clone, Copy, Eq)]
pub struct ReplayFingerprint([u8; 32]);

impl PartialEq for ReplayFingerprint {
    fn eq(&self, other: &Self) -> bool {
        blake3::Hash::from(self.0) == blake3::Hash::from(other.0)
    }
}

macro_rules! public_digest {
    ($ty:ident) => {
        impl $ty {
            pub fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }
            pub fn to_hex(self) -> String {
                blake3::Hash::from(self.0).to_hex().to_string()
            }
        }
        impl std::fmt::Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&blake3::Hash::from(self.0).to_hex(), f)
            }
        }
        impl std::fmt::Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($ty))
                    .field(&self.to_hex())
                    .finish()
            }
        }
        impl std::str::FromStr for $ty {
            type Err = ReplayKeyError;
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                let hash = blake3::Hash::from_hex(text).map_err(|_| {
                    ReplayKeyError::new(
                        ReplayKeyErrorKind::Invalid,
                        concat!("invalid ", stringify!($ty)),
                    )
                })?;
                if hash.to_hex().as_str() != text {
                    return Err(ReplayKeyError::new(
                        ReplayKeyErrorKind::Invalid,
                        concat!(stringify!($ty), " must be canonical lowercase hex"),
                    ));
                }
                Ok(Self(*hash.as_bytes()))
            }
        }
    };
}

public_digest!(ReplayKeyId);
public_digest!(ReplayFingerprint);

/// An in-process credential. Deliberately has no serde or raw-byte public API.
pub struct ReplayKey {
    id: ReplayKeyId,
    pub(super) material: Zeroizing<[u8; 32]>,
}

impl ReplayKey {
    pub fn generate() -> Result<Self, ReplayKeyError> {
        let mut material = Zeroizing::new([0; 32]);
        getrandom::fill(material.as_mut()).map_err(|_| {
            ReplayKeyError::new(
                ReplayKeyErrorKind::Unavailable,
                "cannot generate checkpoint replay key",
            )
        })?;
        Ok(Self::from_secret(material))
    }

    /// For trusted host providers importing high-entropy key material.
    pub fn from_bytes(material: [u8; 32]) -> Self {
        Self::from_secret(Zeroizing::new(material))
    }

    pub(super) fn from_secret(material: Zeroizing<[u8; 32]>) -> Self {
        let id = ReplayKeyId(blake3::derive_key(
            "etas.host.replay.key-id.v1",
            material.as_ref(),
        ));
        Self { id, material }
    }

    pub fn id(&self) -> ReplayKeyId {
        self.id
    }

    pub fn fingerprint(
        &self,
        payload: &HostTracePayload,
    ) -> Result<ReplayFingerprint, ReplayKeyError> {
        payload
            .keyed_digest(&self.material, b"etas.host.replay.request.v1")
            .map(|hash| ReplayFingerprint(*hash.as_bytes()))
            .map_err(|_| {
                ReplayKeyError::new(
                    ReplayKeyErrorKind::Invalid,
                    "invalid canonical replay request payload",
                )
            })
    }
}

impl std::fmt::Debug for ReplayKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayKey")
            .field("id", &self.id)
            .field("material", &"<redacted>")
            .finish()
    }
}
