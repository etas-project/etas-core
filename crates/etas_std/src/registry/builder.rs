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

    pub fn source_parameter_interface(
        &mut self,
        callable: etas_code::CallableId,
        parameters: Vec<super::StdParameterInterface>,
    ) {
        if let Some(previous) = self
            .registry
            .parameter_interfaces
            .insert(callable, parameters.clone())
        {
            assert_eq!(previous, parameters, "one checked interface per callable");
        }
    }

    /// Publish only a role read from an admitted source Runtime declaration.
    pub fn source_execution_role(
        &mut self,
        callable: etas_code::CallableId,
        role: crate::StdExecutionRole,
    ) {
        assert!(
            role.accepts_identity(&callable),
            "source execution role requires its canonical callable identity"
        );
        if let Some(previous) = self.registry.execution_roles.insert(callable, role) {
            assert_eq!(previous, role, "source callable has one execution contract");
        }
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

    /// Add a source overload together with its own checked body identity.
    pub fn source_flow_overload(
        &mut self,
        symbol: StdSymbolId,
        declaration: crate::FlowDecl,
        callable: etas_code::CallableId,
    ) -> Result<(), StdRegistryValidationError> {
        assert!(
            self.registry.body_binding(symbol).is_some(),
            "source overload owner"
        );
        self.flow_overload(symbol, declaration)?;
        let body = self.registry.bodies.last_key_value().map_or(0, |(id, _)| {
            id.checked_add(1).expect("std body identity capacity")
        });
        self.registry
            .bodies
            .insert(body, crate::StdBodyBinding { callable });
        self.registry
            .flow_overload_bodies
            .entry(symbol)
            .or_default()
            .push(body);
        Ok(())
    }

    /// Publish a checked source signature with its validated primitive binding.
    pub fn source_primitive_flow(
        &mut self,
        symbol: StdSymbolId,
        module: StdModuleId,
        declaration: crate::FlowDecl,
        definition: etas_code::DefinitionKey,
    ) {
        let mut path = self.registry.module(module).unwrap().path.clone();
        path.push(declaration.name.clone());
        let entry = self.registry.symbols[symbol.0 as usize].as_mut().unwrap();
        assert!(
            matches!(entry.decl, StdDecl::Flow(_)) && entry.intrinsic.is_some(),
            "checked native flow binding"
        );
        entry.module = module;
        entry.name = declaration.name.clone();
        entry.qualified_path = path.clone();
        entry.decl = StdDecl::Flow(declaration);
        // Public paths belong to the source package. Native contract paths
        // remain in the intrinsic ABI; source re-exports are published later.
        self.registry.qualified.retain(|_, id| *id != symbol);
        self.registry.qualified.insert(path, symbol);
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
    /// permissions remain execution contracts; source owns default Effect rows.
    pub fn source_action_signature(
        &mut self,
        symbol: StdSymbolId,
        signature: crate::EffectActionDecl,
    ) {
        let StdDecl::EffectAction(action) = &mut self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .unwrap()
            .decl
        else {
            unreachable!("checked native action binding");
        };
        action.default_handler_effect_tail = signature
            .default_handler_effects
            .as_ref()
            .and(signature.default_handler_effect_tail);
        action.default_handler_effects = signature.default_handler_effects;
        action.type_params = signature.type_params;
        action.effect_args = signature.effect_args;
        action.selector_param_names = signature.selector_param_names;
        action.params = signature.params;
        action.output = signature.output;
    }

    /// Keep execution entries backed by action declarations in this source
    /// library. Removed slots preserve the checked native dispatch addresses.
    pub fn retain_source_actions(&mut self, actions: &std::collections::BTreeSet<StdSymbolId>) {
        let removed = self
            .registry
            .symbols()
            .filter_map(|symbol| {
                (matches!(symbol.decl, StdDecl::EffectAction(_)) && !actions.contains(&symbol.id))
                    .then_some(symbol.id)
            })
            .collect::<std::collections::BTreeSet<_>>();
        for id in &removed {
            self.registry.symbols[id.0 as usize] = None;
        }
        self.registry
            .qualified
            .retain(|_, id| !removed.contains(id));
        self.registry.prelude.remove_symbols(&removed);
        self.registry
            .action_overloads
            .retain(|id, _| !removed.contains(id));
        self.registry
            .declarations
            .retain(|id, _| !removed.contains(id));
    }

    pub fn source_action_overloads(
        &mut self,
        symbol: StdSymbolId,
        overloads: Vec<crate::EffectActionDecl>,
    ) {
        self.registry.action_overloads.insert(symbol, overloads);
    }

    /// Source parts and query projections share one namespace per full path.
    pub fn module(&mut self, path: &[&str], summary: &str) -> StdModuleId {
        if let Some(module) = self.registry.modules().find(|module| {
            module
                .path
                .iter()
                .map(String::as_str)
                .eq(path.iter().copied())
        }) {
            return module.id;
        }
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
        let source = self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .expect("source role symbol");
        let role = crate::StdSourceRole::from_path(&source.qualified_path.join("."));
        let StdDecl::Type(declaration) = &mut source.decl else {
            panic!("checked source type role");
        };
        if let Some(role) = role {
            declaration.canonical_type = role.declaration(parameters.clone()).canonical_type;
        }
        declaration.params = parameters;
    }

    pub fn source_type_shape(
        &mut self,
        symbol: StdSymbolId,
        kind: crate::TypeDeclKind,
        canonical_type: Option<crate::StdType>,
    ) {
        let source = self.registry.symbols[symbol.0 as usize].as_mut().unwrap();
        let StdDecl::Type(declaration) = &mut source.decl else {
            panic!("checked source type declaration");
        };
        declaration.kind = kind;
        declaration.canonical_type = canonical_type;
        if kind == crate::TypeDeclKind::Enum {
            declaration.representation = None;
        }
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
    pub fn source_type_representation(
        &mut self,
        symbol: StdSymbolId,
        representation: crate::StdType,
    ) {
        let StdDecl::Type(declaration) = &mut self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .unwrap()
            .decl
        else {
            unreachable!("checked source type role");
        };
        declaration.representation = Some(representation);
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

    /// Visibility comes from the checked source declaration. The private
    /// symbol remains a type-dependency query, not a module export.
    pub fn source_visibility(
        &mut self,
        symbol: StdSymbolId,
        visibility: crate::StdSymbolVisibility,
    ) {
        self.registry.symbols[symbol.0 as usize]
            .as_mut()
            .unwrap()
            .visibility = visibility;
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
            visibility: crate::StdSymbolVisibility::Public,
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
        let visibility = symbol.visibility;
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
            visibility,
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

    pub fn native_type_reference(&mut self, reference: super::StdNativeTypeReference) {
        let previous = self
            .registry
            .native_type_references
            .insert(reference.path.join("."), reference);
        assert!(
            previous.is_none(),
            "duplicate native type identity reference"
        );
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

    /// The compiler ships execution signatures whose value types are declared
    /// by std source. Source-HIR binding completes those heads before ordinary
    /// `try_finish` validation; registry image publication also validates.
    pub fn finish_bootstrap(self) -> StdRegistry {
        self.registry
    }

    pub fn try_finish(self) -> Result<StdRegistry, StdRegistryValidationError> {
        self.registry.validate()?;
        Ok(self.registry)
    }
}
