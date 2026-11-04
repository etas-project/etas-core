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
