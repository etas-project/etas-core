use crate::{
    CompletionMetadata, DocsMetadata, IntrinsicDescriptor, StdDecl, StdImplementationRef,
    TextCompatibilityDescriptor,
};

etas_core::id_type!(StdModuleId);
etas_core::id_type!(StdSymbolId);

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    etas_core::RetainedSize,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct StdRegistryVersion {
    pub contract: String,
    pub text: TextCompatibilityDescriptor,
}

impl StdRegistryVersion {
    pub fn phase1() -> Self {
        Self {
            contract: "phase1".to_owned(),
            text: TextCompatibilityDescriptor::current(),
        }
    }

    pub fn compatibility_identity(&self) -> String {
        format!("{}|{}", self.contract, self.text.identity())
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, etas_core::RetainedSize, serde::Serialize, serde::Deserialize,
)]
pub struct StdModule {
    pub id: StdModuleId,
    pub path: Vec<String>,
    pub docs: DocsMetadata,
}

#[derive(
    Clone, Debug, PartialEq, Eq, etas_core::RetainedSize, serde::Serialize, serde::Deserialize,
)]
pub struct StdSymbol {
    pub id: StdSymbolId,
    pub module: StdModuleId,
    pub enum_owner: Option<StdSymbolId>,
    pub name: String,
    pub qualified_path: Vec<String>,
    pub kind: StdSymbolKind,
    pub decl: StdDecl,
    pub intrinsic: Option<IntrinsicDescriptor>,
    pub implementation: Option<StdImplementationRef>,
    pub docs: DocsMetadata,
    pub completion: CompletionMetadata,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize, serde::Serialize, serde::Deserialize,
)]
pub enum StdSymbolKind {
    Type,
    Constructor,
    Flow,
    Tool,
    Effect,
    EffectAction,
    Requirement,
    Value,
    Module,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    etas_core::RetainedSize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum StdNamespace {
    Type,
    Value,
    Effect,
    Module,
}

impl StdSymbolKind {
    pub fn namespace(self) -> StdNamespace {
        match self {
            Self::Type => StdNamespace::Type,
            Self::Effect | Self::EffectAction => StdNamespace::Effect,
            Self::Module => StdNamespace::Module,
            Self::Constructor | Self::Flow | Self::Tool | Self::Requirement | Self::Value => {
                StdNamespace::Value
            }
        }
    }
}
