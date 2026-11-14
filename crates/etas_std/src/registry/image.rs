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

const SCHEMA: u32 = 1;

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
    action_overloads: BTreeMap<StdSymbolId, Vec<EffectActionDecl>>,
    bodies: BTreeMap<u32, StdBodyBinding>,
}

impl StdRegistryImage {
    pub fn capture(registry: &StdRegistry) -> Result<Self, StdRegistryValidationError> {
        registry.validate()?;
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
            action_overloads: registry.action_overloads.clone(),
            bodies: registry.bodies.clone(),
        })
    }

    pub fn into_registry(self) -> Result<StdRegistry, StdRegistryValidationError> {
        if self.schema != SCHEMA {
            return Err(invalid("incompatible standard registry image schema"));
        }
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
            action_overloads: self.action_overloads,
            bodies: self.bodies,
        };
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
