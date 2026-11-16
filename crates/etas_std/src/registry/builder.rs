use crate::{
    CompletionMetadata, DocsMetadata, IntrinsicDescriptor, StdDecl, StdImplFact,
    StdImplementationRef, StdModule, StdModuleId, StdRegistry, StdRegistryValidationError,
    StdRegistryVersion, StdSymbol, StdSymbolId, StdSymbolKind,
};

pub struct StdRegistryBuilder {
    registry: StdRegistry,
}

pub(crate) struct StdSymbolImplementation {
    pub intrinsic: Option<IntrinsicDescriptor>,
    pub implementation: StdImplementationRef,
}

impl StdRegistryBuilder {
    pub fn new(version: StdRegistryVersion) -> Self {
        Self {
            registry: StdRegistry::new(version),
        }
    }

    pub fn from_registry(registry: StdRegistry) -> Self {
        Self { registry }
    }

    pub fn source_flow(
        &mut self,
        module: StdModuleId,
        name: &str,
        declaration: crate::FlowDecl,
        callable: etas_code::CallableId,
        summary: &str,
    ) -> StdSymbolId {
        let body = self.registry.bodies.last_key_value().map_or(0, |(id, _)| {
            id.checked_add(1).expect("std body identity capacity")
        });
        self.registry
            .bodies
            .insert(body, crate::StdBodyBinding { callable });
        let mut path = self
            .registry
            .module(module)
            .expect("source module")
            .path
            .clone();
        path.push(name.to_owned());
        if let Some(mut original) = self
            .registry
            .lookup_qualified(&path)
            .cloned()
            .filter(|symbol| symbol.qualified_path == path)
        {
            assert!(
                (matches!(
                    original.implementation,
                    Some(StdImplementationRef::Primitive { .. })
                ) && matches!(original.decl, StdDecl::Flow(_)))
                    || (original.kind == StdSymbolKind::Constructor
                        && matches!(original.decl, StdDecl::Type(_))
                        && crate::StdSourceRole::from_path(&path.join(".")).is_some()),
                "source flow can replace a native flow contract only"
            );
            let id = original.id;
            original.decl = StdDecl::Flow(declaration);
            original.intrinsic = None;
            original.implementation = Some(StdImplementationRef::Artifact { body });
            self.registry.symbols[id.0 as usize] = Some(original);
            return id;
        }
        self.symbol_with_implementation(
            module,
            name,
            StdSymbolKind::Flow,
            StdDecl::Flow(declaration),
            summary,
            (None, Some(StdImplementationRef::Artifact { body })),
        )
    }

    /// Publish a checked source signature with its validated primitive binding.
    pub fn source_primitive_flow(
        &mut self,
        symbol: StdSymbolId,
        declaration: crate::FlowDecl,
        definition: etas_code::DefinitionKey,
    ) {
        let entry = self.registry.symbols[symbol.0 as usize].as_mut().unwrap();
        assert!(
            matches!(entry.decl, StdDecl::Flow(_)) && entry.intrinsic.is_some(),
            "checked native flow binding"
        );
        entry.decl = StdDecl::Flow(declaration);
        self.bind_declaration(symbol, definition);
    }

    pub fn source_requirement(
        &mut self,
        symbol: StdSymbolId,
        parameters: Vec<crate::StdType>,
        definition: etas_code::DefinitionKey,
    ) {
        let entry = self.registry.symbols[symbol.0 as usize].as_mut().unwrap();
        let StdDecl::Requirement(declaration) = &mut entry.decl else {
            panic!("source requirement must bind an existing compiler contract");
        };
        declaration.params = parameters;
        self.bind_declaration(symbol, definition);
    }

    /// Publish checked source request types and selectors. Stable dispatch,
    /// permissions and default-handler effects remain execution contracts.
    pub fn source_action_signature(
        &mut self,
        symbol: StdSymbolId,
        parameters: Vec<crate::StdGenericParam>,
        selectors: Vec<crate::EffectActionArgKind>,
        selector_names: Vec<String>,
        inputs: Vec<crate::StdType>,
        output: crate::StdType,
    ) {
        let StdDecl::EffectAction(action) = &mut self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .unwrap()
            .decl
        else {
            unreachable!("checked native action binding");
        };
        // The host rule names its row by binder position; source owns spelling.
        if let Some(tail) = &action.default_handler_effect_tail {
            let index = action
                .type_params
                .iter()
                .position(|parameter| &parameter.name == tail)
                .expect("validated native default-handler effect binder");
            action.default_handler_effect_tail = Some(parameters[index].name.clone());
        }
        action.type_params = parameters;
        action.effect_args = selectors;
        action.selector_param_names = selector_names;
        action.params = inputs;
        action.output = output;
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

    pub fn bind_declaration(&mut self, symbol: StdSymbolId, key: etas_code::DefinitionKey) {
        assert!(
            self.registry.declarations.insert(symbol, key).is_none(),
            "source declaration already bound"
        );
    }

    /// The source owns role parameter names; the compiler retains its execution role.
    pub fn source_type_parameters(
        &mut self,
        symbol: StdSymbolId,
        parameters: Vec<crate::StdGenericParam>,
    ) {
        let declaration = &mut self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .expect("source role symbol")
            .decl;
        let StdDecl::Type(declaration) = declaration else {
            panic!("checked source type role");
        };
        declaration.params = parameters;
    }

    /// Source declares effect binders and inheritance; execution tags stay native.
    pub fn source_effect_declaration(
        &mut self,
        symbol: StdSymbolId,
        parameters: Vec<crate::StdGenericParam>,
        extends: Vec<String>,
    ) {
        let StdDecl::Effect(declaration) = &mut self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .unwrap()
            .decl
        else {
            unreachable!("checked source effect role");
        };
        declaration.params = parameters;
        declaration.extends = extends;
    }

    /// Projected fields come from the checked source role declaration.
    pub fn source_type_projected_fields(
        &mut self,
        symbol: StdSymbolId,
        fields: Vec<crate::StdRecordField>,
    ) {
        let StdDecl::Type(declaration) = &mut self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .unwrap()
            .decl
        else {
            unreachable!("checked source type role");
        };
        declaration.projected_fields = fields;
    }

    pub fn named_enum_fields(&mut self, constructor: StdSymbolId, names: Vec<String>) {
        assert!(
            self.registry
                .enum_field_names
                .insert(constructor, names)
                .is_none(),
            "enum fields already bound"
        );
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
        let implementation = intrinsic
            .as_ref()
            .map(|descriptor| StdImplementationRef::Primitive {
                intrinsic: descriptor.id,
            });
        self.symbol_with_implementation(
            module,
            name,
            kind,
            decl,
            summary,
            (intrinsic, implementation),
        )
    }

    pub(crate) fn symbol_with_source_implementation(
        &mut self,
        module: StdModuleId,
        name: &str,
        kind: StdSymbolKind,
        decl: StdDecl,
        summary: &str,
        source: StdSymbolImplementation,
    ) -> StdSymbolId {
        self.symbol_with_implementation(
            module,
            name,
            kind,
            decl,
            summary,
            (source.intrinsic, Some(source.implementation)),
        )
    }

    fn symbol_with_implementation(
        &mut self,
        module: StdModuleId,
        name: &str,
        kind: StdSymbolKind,
        decl: StdDecl,
        summary: &str,
        runtime: (Option<IntrinsicDescriptor>, Option<StdImplementationRef>),
    ) -> StdSymbolId {
        let (intrinsic, implementation) = runtime;
        let id = StdSymbolId(self.registry.symbols.len() as u32);
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
            implementation,
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
        let id = StdSymbolId(self.registry.symbols.len() as u32);
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
            implementation: None,
            docs: DocsMetadata::summary(summary),
        });
        Ok(id)
    }

    pub fn prelude(&mut self, name: &str, symbol: StdSymbolId) {
        self.registry.prelude_mut().insert(name, symbol);
    }

    pub fn flow_overload(
        &mut self,
        symbol: StdSymbolId,
        declaration: crate::FlowDecl,
    ) -> Result<(), StdRegistryValidationError> {
        let target = self
            .registry
            .symbol(symbol)
            .ok_or_else(|| StdRegistryValidationError {
                symbol: format!("symbol {}", symbol.0),
                reason: "overload target is not registered".into(),
            })?;
        if !matches!(&target.decl, StdDecl::Flow(_)) || target.name != declaration.name {
            return Err(StdRegistryValidationError {
                symbol: target.qualified_path.join("."),
                reason: "flow overload must preserve its resolved declaration identity".into(),
            });
        }
        self.registry
            .flow_overloads
            .entry(symbol)
            .or_default()
            .push(declaration);
        Ok(())
    }

    pub fn action_overload(
        &mut self,
        symbol: StdSymbolId,
        declaration: crate::EffectActionDecl,
    ) -> Result<(), StdRegistryValidationError> {
        let target = self
            .registry
            .symbol(symbol)
            .ok_or_else(|| StdRegistryValidationError {
                symbol: format!("symbol {}", symbol.0),
                reason: "overload target is not registered".into(),
            })?;
        if !matches!(&target.decl, StdDecl::EffectAction(_)) || target.name != declaration.name {
            return Err(StdRegistryValidationError {
                symbol: target.qualified_path.join("."),
                reason: "action overload must preserve its resolved declaration identity".into(),
            });
        }
        self.registry
            .action_overloads
            .entry(symbol)
            .or_default()
            .push(declaration);
        Ok(())
    }

    pub fn re_export(
        &mut self,
        module: StdModuleId,
        name: &str,
        symbol: StdSymbolId,
    ) -> Result<(), StdRegistryValidationError> {
        let invalid = |reason: &str| StdRegistryValidationError {
            symbol: name.into(),
            reason: reason.into(),
        };
        let module = self
            .registry
            .module(module)
            .ok_or_else(|| invalid("re-export module is not registered"))?;
        if name.is_empty() || name.contains('.') {
            return Err(invalid("re-export name must be a single nonempty segment"));
        }
        let mut path = module.path.clone();
        path.push(name.into());
        if self.registry.symbol(symbol).is_none() {
            return Err(invalid("re-export target is not registered"));
        }
        if self.registry.qualified.contains_key(&path) {
            return Err(invalid("re-export path is already registered"));
        }
        self.registry.qualified.insert(path, symbol);
        Ok(())
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

    pub fn native_enum_reference(&mut self, reference: super::StdNativeEnumReference) {
        let previous = self
            .registry
            .native_enum_references
            .insert(reference.path.join("."), reference);
        assert!(previous.is_none(), "duplicate native enum ABI reference");
    }

    pub fn native_opaque_reference(&mut self, reference: super::StdNativeOpaqueReference) {
        let previous = self
            .registry
            .native_opaque_references
            .insert(reference.path.join("."), reference);
        assert!(previous.is_none(), "duplicate native opaque ABI reference");
    }

    pub fn native_record_reference(&mut self, reference: super::StdNativeRecordReference) {
        let previous = self
            .registry
            .native_record_references
            .insert(reference.path.join("."), reference);
        assert!(previous.is_none(), "duplicate native record ABI reference");
    }

    pub fn native_projection_reference(&mut self, reference: super::StdNativeProjectionReference) {
        let previous = self
            .registry
            .native_projection_references
            .insert(reference.path.join("."), reference);
        assert!(
            previous.is_none(),
            "duplicate native projection ABI reference"
        );
    }

    /// Replace a bootstrap spec's implementations with its checked bundle facts.
    pub fn replace_spec_impls(&mut self, path: &[String], implementations: Vec<StdImplFact>) {
        self.registry
            .spec_impls
            .retain(|implementation| implementation.spec.path != path);
        self.registry.spec_impls.extend(implementations);
    }

    pub fn finish(self) -> StdRegistry {
        self.try_finish().unwrap_or_else(|error| panic!("{error}"))
    }

    pub fn try_finish(self) -> Result<StdRegistry, StdRegistryValidationError> {
        self.registry.validate()?;
        Ok(self.registry)
    }
}
