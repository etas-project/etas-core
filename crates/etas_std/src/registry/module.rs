use crate::{CompletionMetadata, DocsMetadata, IntrinsicDescriptor, StdDecl};

etas_core::id_type!(StdModuleId);
etas_core::id_type!(StdSymbolId);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StdRegistryVersion {
    pub contract: String,
}

impl StdRegistryVersion {
    pub fn phase1() -> Self {
        Self {
            contract: "phase1".to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdModule {
    pub id: StdModuleId,
    pub path: Vec<String>,
    pub docs: DocsMetadata,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StdSymbol {
    pub id: StdSymbolId,
    pub module: StdModuleId,
    pub name: String,
    pub qualified_path: Vec<String>,
    pub kind: StdSymbolKind,
    pub decl: StdDecl,
    pub intrinsic: Option<IntrinsicDescriptor>,
    pub docs: DocsMetadata,
    pub completion: CompletionMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
