use crate::DefinitionKey;

use super::{EffectRowInfoRef, GenericParameterRef, SignatureInfo};

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
pub struct TypeInfoRef(pub u32);

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum PrimitiveInfo {
    Unit,
    Never,
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    Char,
    String,
    Bytes,
}

/// Compiler-defined type constructors, not a second standard-library API table.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum BuiltinTypeInfo {
    Array,
    List,
    Map,
    Set,
    Range,
    Slice,
    Option,
    Result,
    Schema,
    Prompt,
    PromptPart,
    Message,
    MemorySelection,
    Store,
    MemoryRegion,
    MemoryRegionHandle,
    ExternalToolHandle,
    Trusted,
    Untrusted,
    Secret,
    Public,
    Sanitized,
    /// A checked `CallableDef<F>` role supplied by the standard registry.
    CallableDefinition,
}

impl BuiltinTypeInfo {
    pub const fn arity(self) -> usize {
        match self {
            Self::Prompt | Self::PromptPart => 0,
            Self::Map | Self::Result | Self::Store => 2,
            _ => 1,
        }
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct FieldInfo {
    pub name: String,
    pub ty: TypeInfoRef,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum VariantPayloadInfo {
    None,
    Positional(Vec<TypeInfoRef>),
    Named(Vec<FieldInfo>),
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct VariantInfo {
    pub name: String,
    pub payload: VariantPayloadInfo,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum TypeInfo {
    Primitive(PrimitiveInfo),
    Builtin {
        constructor: BuiltinTypeInfo,
        arguments: Vec<TypeInfoRef>,
    },
    Parameter(GenericParameterRef),
    Tuple(Vec<TypeInfoRef>),
    Record(Vec<FieldInfo>),
    Nominal {
        definition: DefinitionKey,
        parameters: Vec<GenericParameterRef>,
        representation: Option<TypeInfoRef>,
    },
    Alias {
        definition: DefinitionKey,
        parameters: Vec<GenericParameterRef>,
        target: TypeInfoRef,
    },
    Enum {
        definition: DefinitionKey,
        parameters: Vec<GenericParameterRef>,
        variants: Vec<VariantInfo>,
    },
    Applied {
        constructor: TypeInfoRef,
        arguments: Vec<TypeInfoRef>,
    },
    Function(SignatureInfo),
    Handler {
        handled: EffectRowInfoRef,
        produced: Option<EffectRowInfoRef>,
        result: Option<TypeInfoRef>,
    },
}
