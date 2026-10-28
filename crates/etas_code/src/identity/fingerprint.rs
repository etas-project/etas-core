use std::fmt;

#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct CodeFingerprint([u8; 32]);

impl CodeFingerprint {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Content identity only; a digest is never evidence that code is checked.
    pub fn of_bytes(domain: &'static str, bytes: &[u8]) -> Self {
        Self::of_parts(domain, std::iter::once(bytes))
    }

    pub fn of_parts<'a>(domain: &'static str, parts: impl IntoIterator<Item = &'a [u8]>) -> Self {
        let mut encoder = super::canonical::CanonicalEncoder::new(domain);
        for part in parts {
            encoder.bytes(part);
        }
        encoder.finish()
    }
}

impl fmt::Display for CodeFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for CodeFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "CodeFingerprint({self})")
    }
}
