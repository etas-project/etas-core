#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub struct TextCompatibilityDescriptor {
    pub public_contract: String,
    pub primitive_abi: String,
    pub source_implementation: String,
    pub unicode_version: String,
    pub unicode_data_fingerprint: String,
    pub unicode_algorithm: String,
    pub target_usize_bits: u32,
}

impl TextCompatibilityDescriptor {
    pub fn current() -> Self {
        Self {
            public_contract: "etas.std.text.contract.v1".into(),
            primitive_abi: "etas.std.text.primitive-abi.v1".into(),
            source_implementation: "etas.std.text.source.v1".into(),
            unicode_version: "17.0.0".into(),
            unicode_data_fingerprint: "7c74aebf116629b6".into(),
            unicode_algorithm: "etas.std.text.unicode.v1".into(),
            target_usize_bits: usize::BITS,
        }
    }

    pub fn identity(&self) -> String {
        format!(
            "{}|{}|{}|unicode:{}:{}:{}|usize:{}",
            self.public_contract,
            self.primitive_abi,
            self.source_implementation,
            self.unicode_version,
            self.unicode_data_fingerprint,
            self.unicode_algorithm,
            self.target_usize_bits,
        )
    }
}

impl Default for TextCompatibilityDescriptor {
    fn default() -> Self {
        Self::current()
    }
}
