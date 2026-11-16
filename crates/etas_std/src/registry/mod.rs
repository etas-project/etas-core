mod action_forwarder;
pub mod builder;
mod canonical_type;
mod checked_index;
mod collection;
mod default_action;
mod enum_constructor;
mod image;
pub mod lookup;
pub mod module;
mod native_reference;
pub mod prelude;
mod validation;
mod value_role;
pub use image::StdRegistryImage;
pub use native_reference::{
    StdNativeEnumReference, StdNativeOpaqueReference, StdNativeProjectionReference,
    StdNativeRecordReference,
};

pub use builder::StdRegistryBuilder;
pub use module::{
    StdModule, StdModuleId, StdNamespace, StdRegistryVersion, StdSymbol, StdSymbolId, StdSymbolKind,
};
pub use prelude::{StdPrelude, StdSymbolRef};
pub use validation::StdRegistryValidationError;

use std::collections::BTreeMap;

use crate::{IntrinsicDescriptor, StdImplFact, StdIntrinsicId};

#[derive(Clone, Debug, Default, etas_core::RetainedSize)]
pub struct StdRegistry {
    version: StdRegistryVersion,
    modules: Vec<StdModule>,
    symbols: Vec<Option<StdSymbol>>,
    bodies: BTreeMap<u32, crate::StdBodyBinding>,
    declarations: BTreeMap<StdSymbolId, etas_code::DefinitionKey>,
    enum_field_names: BTreeMap<StdSymbolId, Vec<String>>,
    prelude: StdPrelude,
    qualified: BTreeMap<Vec<String>, StdSymbolId>,
    intrinsics: BTreeMap<StdIntrinsicId, IntrinsicDescriptor>,
    memory_place_results: BTreeMap<StdIntrinsicId, usize>,
    spec_impls: Vec<StdImplFact>,
    flow_overloads: BTreeMap<StdSymbolId, Vec<crate::FlowDecl>>,
    action_overloads: BTreeMap<StdSymbolId, Vec<crate::EffectActionDecl>>,
    native_enum_references: BTreeMap<String, StdNativeEnumReference>,
    native_opaque_references: BTreeMap<String, StdNativeOpaqueReference>,
    native_record_references: BTreeMap<String, StdNativeRecordReference>,
    native_projection_references: BTreeMap<String, StdNativeProjectionReference>,
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

    /// Bootstrap-only ABI references, resolved by ordinary checked std source.
    pub fn native_enum_references(&self) -> impl Iterator<Item = &StdNativeEnumReference> {
        self.native_enum_references.values()
    }

    pub fn native_enum_reference(&self, path: &str) -> Option<&StdNativeEnumReference> {
        self.native_enum_references.get(path)
    }

    /// A published provider resolves ABI references to checked declarations.
    pub fn validate_resolved_native_enum_references(
        &self,
    ) -> Result<(), StdRegistryValidationError> {
        for reference in self.native_enum_references() {
            let valid = self
                .lookup_qualified(&reference.path)
                .is_some_and(|owner| reference.matches_owner(self, owner));
            if !valid {
                return Err(StdRegistryValidationError {
                    symbol: reference.path.join("."),
                    reason: "native ABI reference has no matching checked source enum".into(),
                });
            }
        }
        Ok(())
    }

    pub fn native_opaque_references(&self) -> impl Iterator<Item = &StdNativeOpaqueReference> {
        self.native_opaque_references.values()
    }

    pub fn native_opaque_reference(&self, path: &str) -> Option<&StdNativeOpaqueReference> {
        self.native_opaque_references.get(path).or_else(|| {
            if path.contains('.') {
                return None;
            }
            let mut matches = self
                .native_opaque_references
                .values()
                .filter(|reference| reference.path.last().is_some_and(|name| name == path));
            let matched = matches.next()?;
            matches.next().is_none().then_some(matched)
        })
    }

    pub fn validate_resolved_native_opaque_references(
        &self,
    ) -> Result<(), StdRegistryValidationError> {
        for reference in self.native_opaque_references() {
            let valid = self
                .lookup_qualified(&reference.path)
                .is_some_and(|owner| reference.matches_declaration(&owner.decl));
            if !valid {
                return Err(StdRegistryValidationError {
                    symbol: reference.path.join("."),
                    reason: "native ABI reference has no matching checked opaque source type"
                        .into(),
                });
            }
        }
        Ok(())
    }

    pub fn native_record_references(&self) -> impl Iterator<Item = &StdNativeRecordReference> {
        self.native_record_references.values()
    }

    pub fn native_record_reference(&self, path: &str) -> Option<&StdNativeRecordReference> {
        self.native_record_references.get(path).or_else(|| {
            if path.contains('.') {
                return None;
            }
            let mut matches = self
                .native_record_references
                .values()
                .filter(|reference| reference.path.last().is_some_and(|name| name == path));
            let reference = matches.next()?;
            matches.next().is_none().then_some(reference)
        })
    }

    pub fn validate_resolved_native_record_references(
        &self,
    ) -> Result<(), StdRegistryValidationError> {
        for reference in self.native_record_references() {
            if !self
                .lookup_qualified(&reference.path)
                .is_some_and(|owner| reference.matches_declaration(&owner.decl))
            {
                return Err(StdRegistryValidationError {
                    symbol: reference.path.join("."),
                    reason: "native ABI reference has no matching checked source record".into(),
                });
            }
        }
        Ok(())
    }

    pub fn native_projection_references(
        &self,
    ) -> impl Iterator<Item = &StdNativeProjectionReference> {
        self.native_projection_references.values()
    }

    pub fn validate_resolved_native_projection_references(
        &self,
    ) -> Result<(), StdRegistryValidationError> {
        for reference in self.native_projection_references() {
            if !self
                .lookup_qualified(&reference.path)
                .is_some_and(|owner| reference.matches_declaration(&owner.decl))
            {
                return Err(StdRegistryValidationError {
                    symbol: reference.path.join("."),
                    reason: "native getter ABI has no matching checked source projections".into(),
                });
            }
        }
        Ok(())
    }

    pub fn modules(&self) -> impl Iterator<Item = &StdModule> {
        self.modules.iter()
    }

    pub fn symbols(&self) -> impl Iterator<Item = &StdSymbol> {
        self.symbols.iter().flatten()
    }

    /// Native/type bootstrap view. Removed source slots preserve the numeric
    /// native ABI addresses shared with the checked library image.
    pub fn native_bootstrap(&self) -> Self {
        let mut registry = self.clone();
        let source_ids = self
            .symbols()
            .filter(|symbol| {
                matches!(
                    symbol.implementation,
                    Some(
                        crate::StdImplementationRef::Source { .. }
                            | crate::StdImplementationRef::Artifact { .. }
                    )
                )
            })
            .map(|symbol| symbol.id)
            .chain(self.declarations.keys().copied())
            .collect::<std::collections::BTreeSet<_>>();
        for id in &source_ids {
            registry.symbols[id.0 as usize] = None;
        }
        registry.qualified.retain(|_, id| !source_ids.contains(id));
        registry.prelude.remove_symbols(&source_ids);
        registry
            .flow_overloads
            .retain(|id, _| !source_ids.contains(id));
        registry.bodies.clear();
        registry.declarations.clear();
        registry
            .enum_field_names
            .retain(|id, _| !source_ids.contains(id));
        registry
    }

    /// Canonical source owner supplied by the verified library bundle.
    pub fn declaration_key(&self, symbol: StdSymbolId) -> Option<&etas_code::DefinitionKey> {
        self.declarations
            .get(&symbol)
            .or_else(|| self.body_binding(symbol)?.callable.as_declaration())
    }

    pub fn enum_field_names(&self, constructor: StdSymbolId) -> Option<&[String]> {
        self.enum_field_names.get(&constructor).map(Vec::as_slice)
    }

    pub fn body_binding(&self, symbol: StdSymbolId) -> Option<&crate::StdBodyBinding> {
        let crate::StdImplementationRef::Artifact { body } = self.symbol(symbol)?.implementation?
        else {
            return None;
        };
        self.bodies.get(&body)
    }

    /// Module-level declarations and explicit re-exports, retaining canonical identity.
    pub fn module_exports(&self, module: StdModuleId) -> impl Iterator<Item = (&str, &StdSymbol)> {
        let module = self.module(module);
        self.qualified.iter().filter_map(move |(path, id)| {
            let module = module?;
            let (name, parent) = path.split_last()?;
            if parent != module.path {
                return None;
            }
            Some((name.as_str(), self.symbol(*id)?))
        })
    }

    pub fn prelude(&self) -> &StdPrelude {
        &self.prelude
    }

    pub fn module(&self, id: StdModuleId) -> Option<&StdModule> {
        self.modules.get(id.0 as usize)
    }

    pub fn symbol(&self, id: StdSymbolId) -> Option<&StdSymbol> {
        self.symbols.get(id.0 as usize).and_then(Option::as_ref)
    }

    /// All static signatures share one resolved symbol and intrinsic identity.
    pub fn flow_declarations(&self, id: StdSymbolId) -> impl Iterator<Item = &crate::FlowDecl> {
        let primary = self.symbol(id).and_then(|symbol| match &symbol.decl {
            crate::StdDecl::Flow(flow) => Some(flow),
            _ => None,
        });
        primary
            .into_iter()
            .chain(self.flow_overloads.get(&id).into_iter().flatten())
    }

    pub fn action_declarations(
        &self,
        id: StdSymbolId,
    ) -> impl Iterator<Item = &crate::EffectActionDecl> {
        let primary = self.symbol(id).and_then(|symbol| match &symbol.decl {
            crate::StdDecl::EffectAction(action) => Some(action),
            _ => None,
        });
        primary
            .into_iter()
            .chain(self.action_overloads.get(&id).into_iter().flatten())
    }

    pub fn enum_constructor(&self, owner: StdSymbolId, name: &str) -> Option<&StdSymbol> {
        let mut path = self.symbol(owner)?.qualified_path.clone();
        path.push(name.to_owned());
        self.lookup_qualified(&path)
            .filter(|symbol| symbol.enum_owner == Some(owner))
    }

    pub fn enum_constructors(&self, owner: StdSymbolId) -> impl Iterator<Item = &StdSymbol> {
        self.symbols()
            .filter(move |symbol| symbol.enum_owner == Some(owner))
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

    pub fn lookup_prelude_in_namespace(
        &self,
        name: &str,
        namespace: StdNamespace,
    ) -> Option<&StdSymbol> {
        let mut candidates = self.prelude.candidates(name).filter_map(|binding| {
            self.symbol(binding.id)
                .filter(|symbol| symbol.kind.namespace() == namespace)
        });
        let symbol = candidates.next()?;
        candidates.next().is_none().then_some(symbol)
    }

    pub fn intrinsic(&self, id: StdIntrinsicId) -> Option<&IntrinsicDescriptor> {
        self.intrinsics.get(&id)
    }

    /// The result retains the Store identity of this argument without granting access.
    pub fn memory_place_result_argument(&self, symbol: StdSymbolId) -> Option<usize> {
        let intrinsic = self.symbol(symbol)?.intrinsic.as_ref()?;
        self.memory_place_results.get(&intrinsic.id).copied()
    }

    pub fn spec_impls(&self) -> impl Iterator<Item = &StdImplFact> {
        self.spec_impls.iter()
    }

    pub fn validate(&self) -> Result<(), StdRegistryValidationError> {
        validation::validate_registry(self)
    }

    pub(crate) fn push_module(&mut self, module: StdModule) {
        self.modules.push(module);
    }

    pub(crate) fn push_symbol(&mut self, symbol: StdSymbol) {
        self.qualified
            .entry(symbol.qualified_path.clone())
            .or_insert(symbol.id);
        self.symbols.push(Some(symbol));
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

    pub(crate) fn push_spec_impl(&mut self, implementation: StdImplFact) {
        self.spec_impls.push(implementation);
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
