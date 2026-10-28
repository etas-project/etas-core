use super::{CodeFingerprint, IDENTITY_SCHEMA_VERSION};

pub(crate) struct CanonicalEncoder(blake3::Hasher);

impl CanonicalEncoder {
    pub fn new(domain: &'static str) -> Self {
        let mut value = Self(blake3::Hasher::new_derive_key(domain));
        value.0.update(&IDENTITY_SCHEMA_VERSION.to_le_bytes());
        value
    }

    pub fn tag(&mut self, value: u8) {
        self.0.update(&[value]);
    }

    pub fn number(&mut self, value: u64) {
        self.0.update(&value.to_le_bytes());
    }

    pub fn bytes(&mut self, value: &[u8]) {
        self.0.update(&(value.len() as u64).to_le_bytes());
        self.0.update(value);
    }

    pub fn path(&mut self, value: &[String]) {
        self.0.update(&(value.len() as u64).to_le_bytes());
        for segment in value {
            self.bytes(segment.as_bytes());
        }
    }

    pub fn fingerprint(&mut self, value: CodeFingerprint) {
        self.0.update(value.as_bytes());
    }

    pub fn finish(self) -> CodeFingerprint {
        CodeFingerprint::from_bytes(*self.0.finalize().as_bytes())
    }
}
