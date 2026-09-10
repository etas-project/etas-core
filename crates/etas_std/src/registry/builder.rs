use crate::{
    CompletionMetadata, DocsMetadata, IntrinsicDescriptor, StdDecl, StdImplFact, StdModule,
    StdModuleId, StdRegistry, StdRegistryValidationError, StdRegistryVersion, StdSymbol,
    StdSymbolId, StdSymbolKind,
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
            enum_owner: None,
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

    /// Register a constructor in its enum's namespace, not the enclosing module.
    pub fn enum_constructor(
        &mut self,
        owner: StdSymbolId,
        declaration: crate::FlowDecl,
        summary: &str,
    ) -> Result<StdSymbolId, StdRegistryValidationError> {
        let symbol = self
            .registry
            .symbol(owner)
            .ok_or_else(|| StdRegistryValidationError {
                symbol: format!("enum symbol {}", owner.0),
                reason: "constructor owner is not registered".into(),
            })?;
        if !matches!(&symbol.decl, StdDecl::Type(decl) if decl.kind == crate::TypeDeclKind::Enum) {
            return Err(StdRegistryValidationError {
                symbol: symbol.qualified_path.join("."),
                reason: "constructor owner must be an enum".into(),
            });
        }
        let module = symbol.module;
        let mut qualified_path = symbol.qualified_path.clone();
        qualified_path.push(declaration.name.clone());
        let id = StdSymbolId(self.registry.symbols().count() as u32);
        self.registry.push_symbol(StdSymbol {
            id,
            module,
            enum_owner: Some(owner),
            name: declaration.name.clone(),
            qualified_path,
            kind: StdSymbolKind::Constructor,
            completion: CompletionMetadata::new(&declaration.name, summary),
            decl: StdDecl::Flow(declaration),
            intrinsic: None,
            docs: DocsMetadata::summary(summary),
        });
        Ok(id)
    }

    pub fn prelude(&mut self, name: &str, symbol: StdSymbolId) {
        self.registry.prelude_mut().insert(name, symbol);
    }

    pub fn memory_place_result(
        &mut self,
        symbol: StdSymbolId,
        argument: usize,
    ) -> Result<(), StdRegistryValidationError> {
        let data = self
            .registry
            .symbol(symbol)
            .ok_or_else(|| StdRegistryValidationError {
                symbol: format!("symbol {}", symbol.0),
                reason: "memory provenance producer is not registered".into(),
            })?;
        let invalid = |reason: &str| StdRegistryValidationError {
            symbol: data.qualified_path.join("."),
            reason: reason.into(),
        };
        let StdDecl::Flow(flow) = &data.decl else {
            return Err(invalid("memory provenance producer must be a callable"));
        };
        let Some(intrinsic) = &data.intrinsic else {
            return Err(invalid(
                "memory provenance producer must have a checked intrinsic",
            ));
        };
        if !matches!(
            flow.params.get(argument),
            Some(crate::StdType::Store { .. })
        ) {
            return Err(invalid(
                "memory provenance producer must have a checked intrinsic and Store argument",
            ));
        }
        if self.registry.memory_place_result_argument(symbol).is_some() {
            return Err(invalid("memory provenance producer is already registered"));
        }
        self.registry
            .memory_place_results
            .insert(intrinsic.id, argument);
        Ok(())
    }

    pub fn spec_impl(&mut self, implementation: StdImplFact) {
        self.registry.push_spec_impl(implementation);
    }

    pub fn finish(self) -> StdRegistry {
        self.try_finish().unwrap_or_else(|error| panic!("{error}"))
    }

    pub fn try_finish(self) -> Result<StdRegistry, StdRegistryValidationError> {
        self.registry.validate()?;
        Ok(self.registry)
    }
}
