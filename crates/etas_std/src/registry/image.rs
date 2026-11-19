//! Portable query data. Deserialization alone never publishes a StdRegistry;
//! `into_registry` validates contracts and all bundle-local references.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{
    StdModule, StdPrelude, StdRegistry, StdRegistryValidationError, StdRegistryVersion, StdSymbol,
    StdSymbolId,
};
use crate::{
    EffectActionDecl, FlowDecl, IntrinsicDescriptor, StdBodyBinding, StdImplFact,
    StdImplementationRef, StdIntrinsicId,
};

const SCHEMA: u32 = 14;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StdRegistryImage {
    schema: u32,
    version: StdRegistryVersion,
    modules: Vec<StdModule>,
    symbols: Vec<Option<StdSymbol>>,
    prelude: StdPrelude,
    // JSON and CBOR both retain the same canonical order for qualified paths.
    qualified: Vec<(Vec<String>, StdSymbolId)>,
    intrinsics: BTreeMap<StdIntrinsicId, IntrinsicDescriptor>,
    memory_place_results: BTreeMap<StdIntrinsicId, usize>,
    spec_impls: Vec<StdImplFact>,
    flow_overloads: BTreeMap<StdSymbolId, Vec<FlowDecl>>,
    flow_overload_bodies: BTreeMap<StdSymbolId, Vec<u32>>,
    action_overloads: BTreeMap<StdSymbolId, Vec<EffectActionDecl>>,
    bodies: BTreeMap<u32, StdBodyBinding>,
    execution_roles: Vec<(etas_code::CallableId, crate::StdExecutionRole)>,
    parameter_interfaces: Vec<(etas_code::CallableId, Vec<super::StdParameterInterface>)>,
    declarations: BTreeMap<StdSymbolId, etas_code::DefinitionKey>,
    enum_field_names: BTreeMap<StdSymbolId, Vec<String>>,
    native_enum_references: BTreeMap<String, super::StdNativeEnumReference>,
    native_opaque_references: BTreeMap<String, super::StdNativeOpaqueReference>,
    native_record_references: BTreeMap<String, super::StdNativeRecordReference>,
    native_projection_references: BTreeMap<String, super::StdNativeProjectionReference>,
    native_type_references: BTreeMap<String, super::StdNativeTypeReference>,
}

impl StdRegistryImage {
    pub fn capture(registry: &StdRegistry) -> Result<Self, StdRegistryValidationError> {
        registry.validate()?;
        for reference in registry.native_type_references() {
            if let Some(owner) = registry.lookup_qualified_internal(&reference.path)
                && !reference.matches_declaration(&owner.decl)
            {
                return Err(invalid(
                    "checked source type disagrees with its native constructor import",
                ));
            }
        }
        for reference in registry.native_enum_references() {
            let Some(owner) = registry.lookup_qualified_internal(&reference.path) else {
                continue;
            };
            if registry.declaration_key(owner.id).is_none()
                && crate::StdSourceRole::from_path(&reference.path.join(".")).is_some()
            {
                continue; // The checked source publishes this role's constructors.
            }
            if !reference.matches_owner(registry, owner) {
                return Err(invalid(
                    "checked source enum disagrees with its native ABI reference",
                ));
            }
        }
        for reference in registry.native_opaque_references() {
            if let Some(owner) = registry.lookup_qualified_internal(&reference.path) {
                if !reference.matches_declaration(&owner.decl) {
                    return Err(invalid(
                        "checked opaque source type disagrees with its native ABI reference",
                    ));
                }
            }
        }
        for reference in registry.native_record_references() {
            if let Some(owner) = registry.lookup_qualified_internal(&reference.path) {
                if !reference.matches_declaration(&owner.decl) {
                    return Err(invalid(
                        "checked source record disagrees with its native ABI reference",
                    ));
                }
            }
        }
        for reference in registry.native_projection_references() {
            if let Some(owner) = registry.lookup_qualified_internal(&reference.path)
                && registry.declaration_key(owner.id).is_some()
                && !reference.matches_declaration(&owner.decl)
            {
                return Err(invalid(
                    "checked source projections disagree with their native getter ABI",
                ));
            }
        }
        if registry.symbols().any(|symbol| {
            matches!(
                symbol.implementation,
                Some(StdImplementationRef::Source { .. })
            )
        }) {
            return Err(invalid(
                "artifact registry still contains a legacy static source implementation",
            ));
        }
        Ok(Self {
            schema: SCHEMA,
            version: registry.version.clone(),
            modules: registry.modules.clone(),
            symbols: registry.symbols.clone(),
            prelude: registry.prelude.clone(),
            qualified: registry
                .qualified
                .iter()
                .map(|(path, id)| (path.clone(), *id))
                .collect(),
            intrinsics: registry.intrinsics.clone(),
            memory_place_results: registry.memory_place_results.clone(),
            spec_impls: registry.spec_impls.clone(),
            flow_overloads: registry.flow_overloads.clone(),
            flow_overload_bodies: registry.flow_overload_bodies.clone(),
            action_overloads: registry.action_overloads.clone(),
            bodies: registry.bodies.clone(),
            execution_roles: registry
                .execution_roles
                .iter()
                .map(|(callable, role)| (callable.clone(), *role))
                .collect(),
            declarations: registry.declarations.clone(),
            parameter_interfaces: registry
                .parameter_interfaces
                .iter()
                .map(|(callable, parameters)| (callable.clone(), parameters.clone()))
                .collect(),
            enum_field_names: registry.enum_field_names.clone(),
            native_enum_references: registry.native_enum_references.clone(),
            native_opaque_references: registry.native_opaque_references.clone(),
            native_record_references: registry.native_record_references.clone(),
            native_projection_references: registry.native_projection_references.clone(),
            native_type_references: registry.native_type_references.clone(),
        })
    }

    pub fn into_registry(self) -> Result<StdRegistry, StdRegistryValidationError> {
        if self.schema != SCHEMA {
            return Err(invalid("incompatible standard registry image schema"));
        }
        let execution_count = self.execution_roles.len();
        let interface_count = self.parameter_interfaces.len();
        let registry = StdRegistry {
            version: self.version,
            modules: self.modules,
            symbols: self.symbols,
            prelude: self.prelude,
            qualified: self.qualified.into_iter().collect(),
            intrinsics: self.intrinsics,
            memory_place_results: self.memory_place_results,
            spec_impls: self.spec_impls,
            flow_overloads: self.flow_overloads,
            flow_overload_bodies: self.flow_overload_bodies,
            action_overloads: self.action_overloads,
            bodies: self.bodies,
            execution_roles: self.execution_roles.into_iter().collect(),
            parameter_interfaces: self.parameter_interfaces.into_iter().collect(),
            declarations: self.declarations,
            enum_field_names: self.enum_field_names,
            native_enum_references: self.native_enum_references,
            native_opaque_references: self.native_opaque_references,
            native_record_references: self.native_record_references,
            native_projection_references: self.native_projection_references,
            native_type_references: self.native_type_references,
        };
        if registry.execution_roles.len() != execution_count {
            return Err(invalid("duplicate source execution role identity"));
        }
        if registry.parameter_interfaces.len() != interface_count {
            return Err(invalid("duplicate source parameter interface identity"));
        }
        for (index, module) in registry.modules.iter().enumerate() {
            if module.id.0 as usize != index {
                return Err(invalid("module slot disagrees with its identity"));
            }
        }
        for (index, symbol) in registry.symbols.iter().enumerate() {
            if symbol
                .as_ref()
                .is_some_and(|symbol| symbol.id.0 as usize != index)
            {
                return Err(invalid("symbol slot disagrees with its identity"));
            }
            if symbol
                .as_ref()
                .is_some_and(|symbol| registry.module(symbol.module).is_none())
            {
                return Err(invalid("declaration references a missing module"));
            }
        }
        for (id, descriptor) in &registry.intrinsics {
            if *id != descriptor.id {
                return Err(invalid("native binding slot disagrees with its identity"));
            }
        }
        for symbol in registry.symbols() {
            if let Some(descriptor) = &symbol.intrinsic {
                let binding = registry
                    .intrinsic(descriptor.id)
                    .ok_or_else(|| invalid("declaration references a missing native binding"))?;
                if !super::intrinsic_semantics_match(binding, descriptor) {
                    return Err(invalid("declaration disagrees with its native binding ABI"));
                }
            }
        }
        for (path, id) in &registry.qualified {
            if path.is_empty() || registry.symbol(*id).is_none() {
                return Err(invalid("export references a missing standard declaration"));
            }
        }
        registry.validate()?;
        Ok(registry)
    }
}

fn invalid(reason: &str) -> StdRegistryValidationError {
    StdRegistryValidationError {
        symbol: "<std registry image>".into(),
        reason: reason.into(),
    }
}
