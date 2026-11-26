use super::DescriptorError;
use crate::{CodeFingerprint, DefinitionKey, DefinitionKind, MethodId};

#[derive(
    Clone,
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
pub enum ParameterDefaultOwner {
    Declaration(DefinitionKey),
    Method(MethodId),
    SpecMethod { spec: DefinitionKey, name: String },
    Action { effect: DefinitionKey, name: String },
}

impl ParameterDefaultOwner {
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = crate::identity::canonical::CanonicalEncoder::new(
            "etas.code.parameter-default-owner.v1",
        );
        match self {
            Self::Declaration(key) => {
                encoder.tag(0);
                encoder.fingerprint(key.fingerprint());
            }
            Self::Method(method) => {
                encoder.tag(1);
                encoder.fingerprint(method.fingerprint());
            }
            Self::SpecMethod { spec, name } => {
                encoder.tag(2);
                encoder.fingerprint(spec.fingerprint());
                encoder.bytes(name.as_bytes());
            }
            Self::Action { effect, name } => {
                encoder.tag(3);
                encoder.fingerprint(effect.fingerprint());
                encoder.bytes(name.as_bytes());
            }
        }
        encoder.finish()
    }

    pub fn validate(&self) -> Result<(), DescriptorError> {
        let valid = match self {
            Self::Declaration(key) => matches!(
                key.kind(),
                DefinitionKind::Flow | DefinitionKind::Agent | DefinitionKind::Tool
            ),
            Self::Method(_) => true,
            Self::SpecMethod { spec, name } => {
                spec.kind() == DefinitionKind::Spec && !name.is_empty()
            }
            Self::Action { effect, name } => {
                effect.kind() == DefinitionKind::Effect && !name.is_empty()
            }
        };
        if valid {
            Ok(())
        } else {
            Err(DescriptorError::InvalidDescription {
                reason: "parameter default has an invalid declaration owner",
            })
        }
    }
}

/// Portable reference to an authored, checked default expression revision.
/// Its body and lexical environment belong to the defining compiled package.
#[derive(
    Clone,
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
#[serde(deny_unknown_fields)]
pub struct ParameterDefaultInfo {
    /// This binding requests a provider rather than evaluating a pure default.
    /// The checked expression revision retains the provider identity and label.
    #[serde(default)]
    pub injected: bool,
    pub owner: ParameterDefaultOwner,
    pub parameter: u32,
    pub revision: CodeFingerprint,
}

impl ParameterDefaultInfo {
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder =
            crate::identity::canonical::CanonicalEncoder::new("etas.code.parameter-default.v2");
        encoder.fingerprint(self.owner.fingerprint());
        encoder.number(u64::from(self.parameter));
        encoder.fingerprint(self.revision);
        encoder.number(u64::from(self.injected));
        encoder.finish()
    }
}
