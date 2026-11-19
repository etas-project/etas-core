use crate::{DefinitionKey, ImplementationKey, MethodId};

use super::{EffectRowInfoRef, TypeInfoRef};

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
pub struct GenericParameterRef(pub u32);

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
pub enum GenericOwner {
    Declaration(DefinitionKey),
    Method(MethodId),
    Implementation(ImplementationKey),
    /// Header-local binders, before its implementation identity is computed.
    ImplementationPattern(DefinitionKey),
    /// Signature binders of an action, separate from its static selectors.
    Action {
        effect: DefinitionKey,
        name: String,
    },
    /// A checked input signature distinguishes declarations in an action group.
    ActionOverload {
        effect: DefinitionKey,
        name: String,
        signature: crate::CodeFingerprint,
    },
}

impl GenericOwner {
    pub fn is_portable_parameter_owner(&self) -> bool {
        match self {
            Self::Declaration(owner) => matches!(
                owner.kind(),
                crate::DefinitionKind::Type
                    | crate::DefinitionKind::Spec
                    | crate::DefinitionKind::Agent
                    | crate::DefinitionKind::Flow
                    | crate::DefinitionKind::Tool
            ),
            Self::Method(_) | Self::Implementation(_) => true,
            Self::ImplementationPattern(_) => false,
            Self::Action { effect, name } | Self::ActionOverload { effect, name, .. } => {
                effect.kind() == crate::DefinitionKind::Effect && !name.is_empty()
            }
        }
    }

    pub fn fingerprint(&self) -> crate::CodeFingerprint {
        let mut encoder =
            crate::identity::canonical::CanonicalEncoder::new("etas.code.generic-owner.v1");
        let (kind, identity) = match self {
            Self::Declaration(owner) => (0, owner.fingerprint()),
            Self::Method(owner) => (1, owner.fingerprint()),
            Self::Implementation(owner) => (2, owner.fingerprint()),
            Self::ImplementationPattern(owner) => (3, owner.fingerprint()),
            Self::Action { effect, name } => {
                encoder.tag(4);
                encoder.fingerprint(effect.fingerprint());
                encoder.bytes(name.as_bytes());
                return encoder.finish();
            }
            Self::ActionOverload {
                effect,
                name,
                signature,
            } => {
                encoder.tag(5);
                encoder.fingerprint(effect.fingerprint());
                encoder.bytes(name.as_bytes());
                encoder.fingerprint(*signature);
                return encoder.finish();
            }
        };
        encoder.tag(kind);
        encoder.fingerprint(identity);
        encoder.finish()
    }

    pub(crate) fn alpha_scoped(&self) -> bool {
        matches!(
            self,
            Self::Implementation(_) | Self::ImplementationPattern(_)
        )
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum GenericParameterKind {
    Type,
    EffectRow,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct SpecInfo {
    pub definition: DefinitionKey,
    pub arguments: Vec<TypeInfoRef>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct GenericParameterInfo {
    pub owner: GenericOwner,
    pub position: u32,
    pub name: String,
    pub kind: GenericParameterKind,
    pub bounds: Vec<SpecInfo>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct ParameterInfo {
    #[serde(default)]
    pub default: Option<super::ParameterDefaultInfo>,
    pub name: Option<String>,
    pub ty: TypeInfoRef,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct SignatureInfo {
    pub generics: Vec<GenericParameterRef>,
    pub parameters: Vec<ParameterInfo>,
    pub output: TypeInfoRef,
    pub declared_output: Option<TypeInfoRef>,
    pub effects: Option<EffectRowInfoRef>,
}
