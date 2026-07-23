use crate::{
    CompletionMetadata, DocsMetadata, IntrinsicDescriptor, StdDecl, StdModule, StdModuleId,
    StdRegistry, StdRegistryVersion, StdSymbol, StdSymbolId, StdSymbolKind,
};

pub struct StdRegistryBuilder {
    registry: StdRegistry,
}

impl StdRegistryBuilder {
    pub fn new(version: StdRegistryVersion) -> Self {
        Self {
            registry: StdRegistry::new(version),
        }
    }

    pub fn module(&mut self, path: &[&str], summary: &str) -> StdModuleId {
        let id = StdModuleId(self.registry.modules().count() as u32);
        self.registry.push_module(StdModule {
            id,
            path: path.iter().map(|segment| (*segment).to_owned()).collect(),
            docs: DocsMetadata::summary(summary),
        });
        id
    }

    pub fn symbol(
        &mut self,
        module: StdModuleId,
        name: &str,
        kind: StdSymbolKind,
        decl: StdDecl,
        summary: &str,
    ) -> StdSymbolId {
        self.symbol_with_intrinsic(module, name, kind, decl, summary, None)
    }

    pub fn symbol_with_intrinsic(
        &mut self,
        module: StdModuleId,
        name: &str,
        kind: StdSymbolKind,
        decl: StdDecl,
        summary: &str,
        intrinsic: Option<IntrinsicDescriptor>,
    ) -> StdSymbolId {
        let id = StdSymbolId(self.registry.symbols().count() as u32);
        let module_path = self
            .registry
            .module(module)
            .expect("std module id should be valid while building")
            .path
            .clone();
        let mut qualified_path = module_path;
        qualified_path.push(name.to_owned());
        if let Some(descriptor) = &intrinsic {
            assert_eq!(
                descriptor.qualified_path, qualified_path,
                "standard intrinsic descriptor path must match its registered symbol"
            );
            self.registry.push_intrinsic(descriptor.clone());
        }
        self.registry.push_symbol(StdSymbol {
            id,
            module,
            name: name.to_owned(),
            qualified_path,
            kind,
            decl,
            intrinsic,
            docs: DocsMetadata::summary(summary),
            completion: CompletionMetadata::new(name, summary),
        });
        id
    }

    pub fn prelude(&mut self, name: &str, symbol: StdSymbolId) {
        self.registry.prelude_mut().insert(name, symbol);
    }

    pub fn finish(self) -> StdRegistry {
        self.registry
    }
}
