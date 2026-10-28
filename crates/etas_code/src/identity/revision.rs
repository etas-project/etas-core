use super::{CallableId, CodeFingerprint, canonical::CanonicalEncoder};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct RevisionKey(CodeFingerprint);

impl RevisionKey {
    /// Descriptive revision identity, never checked execution evidence.
    pub const fn from_canonical_fingerprint(fingerprint: CodeFingerprint) -> Self {
        Self(fingerprint)
    }
    pub const fn fingerprint(self) -> CodeFingerprint {
        self.0
    }
    pub fn from_inputs(inputs: &RevisionInputs) -> Self {
        let mut encoder = CanonicalEncoder::new("etas.code.revision.v1");
        encoder.fingerprint(inputs.definition.fingerprint());
        encoder.fingerprint(inputs.content);
        encoder.fingerprint(inputs.environment);
        encoder.fingerprint(inputs.contract);
        encoder.fingerprint(inputs.compilation);
        match inputs.parent {
            Some(parent) => {
                encoder.tag(1);
                encoder.fingerprint(parent.fingerprint());
            }
            None => encoder.tag(0),
        }
        Self(encoder.finish())
    }
}

/// Immutable check/compile coordinates, not a compatibility judgment or permit.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
#[serde(deny_unknown_fields)]
pub struct RevisionInputs {
    pub definition: CallableId,
    pub content: CodeFingerprint,
    pub environment: CodeFingerprint,
    pub contract: CodeFingerprint,
    /// Compiler/backend/schema and semantic configuration identity.
    pub compilation: CodeFingerprint,
    pub parent: Option<RevisionKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct RevisionInfo {
    key: RevisionKey,
    inputs: RevisionInputs,
}

impl RevisionInfo {
    pub fn new(inputs: RevisionInputs) -> Self {
        Self {
            key: RevisionKey::from_inputs(&inputs),
            inputs,
        }
    }
    pub fn key(&self) -> RevisionKey {
        self.key
    }
    pub fn inputs(&self) -> &RevisionInputs {
        &self.inputs
    }
}
