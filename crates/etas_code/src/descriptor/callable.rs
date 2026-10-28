use crate::{DefinitionKey, MethodId};

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
