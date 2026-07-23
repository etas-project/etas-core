pub mod builder;
pub mod lookup;
pub mod module;
pub mod prelude;

pub use builder::StdRegistryBuilder;
pub use module::{
    StdModule, StdModuleId, StdRegistryVersion, StdSymbol, StdSymbolId, StdSymbolKind,
};
pub use prelude::{StdPrelude, StdSymbolRef};

use std::collections::BTreeMap;

use crate::{IntrinsicDescriptor, StdIntrinsicId};

#[derive(Clone, Debug, Default)]
pub struct StdRegistry {
    version: StdRegistryVersion,
    modules: Vec<StdModule>,
    symbols: Vec<StdSymbol>,
    prelude: StdPrelude,
    qualified: BTreeMap<Vec<String>, StdSymbolId>,
    intrinsics: BTreeMap<StdIntrinsicId, IntrinsicDescriptor>,
}

impl StdRegistry {
    pub fn new(version: StdRegistryVersion) -> Self {
        Self {
            version,
            ..Self::default()
        }
    }

    pub fn version(&self) -> &StdRegistryVersion {
        &self.version
    }

    pub fn modules(&self) -> impl Iterator<Item = &StdModule> {
        self.modules.iter()
    }

    pub fn symbols(&self) -> impl Iterator<Item = &StdSymbol> {
        self.symbols.iter()
    }

    pub fn prelude(&self) -> &StdPrelude {
        &self.prelude
    }

    pub fn module(&self, id: StdModuleId) -> Option<&StdModule> {
        self.modules.get(id.0 as usize)
    }

    pub fn symbol(&self, id: StdSymbolId) -> Option<&StdSymbol> {
        self.symbols.get(id.0 as usize)
    }

    pub fn lookup_qualified(&self, path: &[impl AsRef<str>]) -> Option<&StdSymbol> {
        let key = path
            .iter()
            .map(|segment| segment.as_ref().to_owned())
            .collect::<Vec<_>>();
        self.qualified.get(&key).and_then(|id| self.symbol(*id))
    }

    pub fn lookup_prelude(&self, name: &str) -> Option<&StdSymbol> {
        self.prelude
            .lookup(name)
            .and_then(|symbol| self.symbol(symbol.id))
    }

    pub fn intrinsic(&self, id: StdIntrinsicId) -> Option<&IntrinsicDescriptor> {
        self.intrinsics.get(&id)
    }

    pub(crate) fn push_module(&mut self, module: StdModule) {
        self.modules.push(module);
    }

    pub(crate) fn push_symbol(&mut self, symbol: StdSymbol) {
        self.qualified
            .insert(symbol.qualified_path.clone(), symbol.id);
        self.symbols.push(symbol);
    }

    pub(crate) fn prelude_mut(&mut self) -> &mut StdPrelude {
        &mut self.prelude
    }

    pub(crate) fn push_intrinsic(&mut self, descriptor: IntrinsicDescriptor) {
        if let Some(existing) = self.intrinsics.get(&descriptor.id) {
            assert!(
                intrinsic_semantics_match(existing, &descriptor),
                "standard intrinsic id {} has conflicting descriptors for `{}` and `{}`",
                descriptor.id.0,
                existing.qualified_path.join("."),
                descriptor.qualified_path.join(".")
            );
            return;
        }
        self.intrinsics.insert(descriptor.id, descriptor);
    }
}

fn intrinsic_semantics_match(left: &IntrinsicDescriptor, right: &IntrinsicDescriptor) -> bool {
    left.id == right.id
        && left.purity == right.purity
        && left.dispatch == right.dispatch
        && left.lowering == right.lowering
        && left.latent_effect == right.latent_effect
        && left.memory_access == right.memory_access
        && left.runtime_requirement == right.runtime_requirement
}
