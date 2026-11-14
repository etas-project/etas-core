//! Canonical contract inventory for comparing source std against the legacy registry.
//! This is an audit format, not an executable std provider or a registry fallback.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{StdDecl, StdImplementationRef, StdRegistry};

pub const STD_MIGRATION_BASELINE_SCHEMA: &str = "etas.std.migration-baseline.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdMigrationBaseline {
    pub schema: String,
    pub contract_version: String,
    pub text_compatibility: Value,
    pub modules: BTreeMap<String, BTreeMap<String, String>>,
    pub symbols: BTreeMap<String, StdBaselineSymbol>,
    pub prelude: BTreeMap<String, String>,
    pub spec_impls: Vec<Value>,
    pub primitives: BTreeMap<u32, StdPrimitiveBaseline>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdBaselineSymbol {
    pub module: String,
    pub kind: String,
    pub enum_owner: Option<String>,
    /// Includes generic/effect parameters, source methods, type/value roles and opaque construction.
    pub declaration: Value,
    pub overloads: Vec<Value>,
    pub implementation: Option<StdBaselineImplementation>,
    pub binding: Option<u32>,
    pub memory_place_result_argument: Option<usize>,
    pub docs: String,
    pub completion_label: String,
    pub completion_detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StdBaselineImplementation {
    Primitive {
        binding: u32,
    },
    Artifact {
        callable: Value,
    },
    Source {
        definition: String,
        source_path: String,
        source_digest: String,
        determinism: Value,
        optional_acceleration: Option<u32>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdPrimitiveBaseline {
    /// Stable ABI ID and dispatch/purity/lowering/effect/memory contracts.
    pub contract: Value,
    pub required_by: Vec<String>,
    pub accelerates: Vec<String>,
}

impl StdMigrationBaseline {
    pub fn from_registry(registry: &StdRegistry) -> Self {
        let canonical = |id| {
            registry
                .symbol(id)
                .expect("registered std symbol")
                .qualified_path
                .join(".")
        };
        let mut result = Self {
            schema: STD_MIGRATION_BASELINE_SCHEMA.into(),
            contract_version: registry.version().contract.clone(),
            text_compatibility: data(&registry.version().text),
            modules: registry
                .modules()
                .map(|module| {
                    (
                        module.path.join("."),
                        registry
                            .module_exports(module.id)
                            .map(|(name, symbol)| (name.to_owned(), canonical(symbol.id)))
                            .collect(),
                    )
                })
                .collect(),
            symbols: BTreeMap::new(),
            prelude: registry
                .prelude()
                .symbols()
                .map(|binding| {
                    let symbol = registry
                        .symbol(binding.id)
                        .expect("registered prelude symbol");
                    (
                        format!("{:?}:{}", symbol.kind.namespace(), binding.name),
                        canonical(binding.id),
                    )
                })
                .collect(),
            spec_impls: registry.spec_impls().map(data).collect(),
            primitives: BTreeMap::new(),
        };
        result.spec_impls.sort_by_cached_key(Value::to_string);
        for symbol in registry.symbols() {
            let identity = canonical(symbol.id);
            let implementation = symbol
                .implementation
                .map(|implementation| match implementation {
                    StdImplementationRef::Primitive { intrinsic } => {
                        StdBaselineImplementation::Primitive {
                            binding: intrinsic.0,
                        }
                    }
                    StdImplementationRef::Artifact { .. } => StdBaselineImplementation::Artifact {
                        callable: data(
                            &registry
                                .body_binding(symbol.id)
                                .expect("validated body binding")
                                .callable,
                        ),
                    },
                    StdImplementationRef::Source {
                        target,
                        optimized_intrinsic,
                    } => {
                        let source = crate::standard_library_source(target.source);
                        StdBaselineImplementation::Source {
                            definition: source
                                .module_path
                                .iter()
                                .copied()
                                .chain([target.item])
                                .collect::<Vec<_>>()
                                .join("."),
                            source_path: source.virtual_path.into(),
                            source_digest: blake3::hash(source.source.as_bytes())
                                .to_hex()
                                .to_string(),
                            determinism: data(&target.determinism),
                            optional_acceleration: optimized_intrinsic.map(|id| id.0),
                        }
                    }
                });
            if let Some(binding) = &symbol.intrinsic {
                let primitive = result.primitives.entry(binding.id.0).or_insert_with(|| {
                    // An intrinsic can have public aliases. Its ABI identity is the
                    // stable ID, not whichever alias was registered first.
                    let mut contract = data(binding);
                    contract
                        .as_object_mut()
                        .expect("intrinsic descriptor record")
                        .remove("qualified_path");
                    StdPrimitiveBaseline {
                        contract,
                        ..Default::default()
                    }
                });
                match &implementation {
                    Some(StdBaselineImplementation::Source {
                        optional_acceleration: Some(id),
                        ..
                    }) if *id == binding.id.0 => {
                        primitive.accelerates.push(identity.clone());
                    }
                    _ => primitive.required_by.push(identity.clone()),
                }
            }
            let overloads = match &symbol.decl {
                StdDecl::Flow(_) => registry
                    .flow_declarations(symbol.id)
                    .skip(1)
                    .map(data)
                    .collect(),
                StdDecl::EffectAction(_) => registry
                    .action_declarations(symbol.id)
                    .skip(1)
                    .map(data)
                    .collect(),
                _ => Vec::new(),
            };
            result.symbols.insert(
                identity,
                StdBaselineSymbol {
                    module: registry
                        .module(symbol.module)
                        .expect("registered std module")
                        .path
                        .join("."),
                    kind: format!("{:?}", symbol.kind),
                    enum_owner: symbol.enum_owner.map(canonical),
                    declaration: data(&symbol.decl),
                    overloads,
                    implementation,
                    binding: symbol.intrinsic.as_ref().map(|binding| binding.id.0),
                    memory_place_result_argument: registry.memory_place_result_argument(symbol.id),
                    docs: symbol.docs.summary.clone(),
                    completion_label: symbol.completion.label.clone(),
                    completion_detail: symbol.completion.detail.clone(),
                },
            );
        }
        for primitive in result.primitives.values_mut() {
            primitive.required_by.sort();
            primitive.accelerates.sort();
        }
        result
    }

    /// Compare a candidate inventory against this baseline without loading legacy std.
    pub fn compare(&self, candidate: &Self) -> StdMigrationReport {
        let mut report = StdMigrationReport {
            compatibility_matches: self.schema == candidate.schema
                && self.contract_version == candidate.contract_version
                && self.text_compatibility == candidate.text_compatibility,
            modules: BTreeMap::new(),
            prelude_changes: changed_keys(&self.prelude, &candidate.prelude),
            impls_match: self.spec_impls == candidate.spec_impls,
            primitive_changes: changed_keys(&self.primitives, &candidate.primitives),
        };
        for (identity, expected) in &self.symbols {
            let module = report.modules.entry(expected.module.clone()).or_default();
            let Some(actual) = candidate.symbols.get(identity) else {
                module.missing.push(identity.clone());
                continue;
            };
            module.covered.push(identity.clone());
            if expected.kind != actual.kind
                || expected.enum_owner != actual.enum_owner
                || expected.declaration != actual.declaration
                || expected.overloads != actual.overloads
                || expected.memory_place_result_argument != actual.memory_place_result_argument
            {
                module.contract_changes.push(identity.clone());
            }
            if expected.implementation != actual.implementation
                || expected.binding != actual.binding
                || expected
                    .binding
                    .and_then(|id| self.primitives.get(&id))
                    .map(|p| &p.contract)
                    != actual
                        .binding
                        .and_then(|id| candidate.primitives.get(&id))
                        .map(|p| &p.contract)
            {
                module.implementation_changes.push(identity.clone());
            }
        }
        for (identity, symbol) in &candidate.symbols {
            if !self.symbols.contains_key(identity) {
                report
                    .modules
                    .entry(symbol.module.clone())
                    .or_default()
                    .added
                    .push(identity.clone());
            }
        }
        for module in self
            .modules
            .keys()
            .chain(candidate.modules.keys())
            .collect::<BTreeSet<_>>()
        {
            let coverage = report.modules.entry(module.clone()).or_default();
            coverage.missing_module = !candidate.modules.contains_key(module);
            coverage.added_module = !self.modules.contains_key(module);
            coverage.export_changes = changed_keys(
                self.modules.get(module).unwrap_or(&BTreeMap::new()),
                candidate.modules.get(module).unwrap_or(&BTreeMap::new()),
            );
        }
        report
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdMigrationReport {
    pub compatibility_matches: bool,
    pub modules: BTreeMap<String, StdModuleCoverage>,
    pub prelude_changes: Vec<String>,
    pub impls_match: bool,
    pub primitive_changes: Vec<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StdModuleCoverage {
    pub missing_module: bool,
    pub added_module: bool,
    pub covered: Vec<String>,
    pub missing: Vec<String>,
    pub added: Vec<String>,
    pub contract_changes: Vec<String>,
    /// Source-content or binding changes require the separate runtime differential suite.
    pub implementation_changes: Vec<String>,
    pub export_changes: Vec<String>,
}

impl StdMigrationReport {
    pub fn matches(&self) -> bool {
        self.compatibility_matches
            && self.impls_match
            && self.prelude_changes.is_empty()
            && self.primitive_changes.is_empty()
            && self.modules.values().all(|module| {
                !module.missing_module
                    && !module.added_module
                    && module.missing.is_empty()
                    && module.added.is_empty()
                    && module.contract_changes.is_empty()
                    && module.implementation_changes.is_empty()
                    && module.export_changes.is_empty()
            })
    }
}

fn data(value: &impl Serialize) -> Value {
    serde_json::to_value(value).expect("std declarations serialize as data")
}

fn changed_keys<K: Ord + Clone, V: PartialEq>(
    expected: &BTreeMap<K, V>,
    actual: &BTreeMap<K, V>,
) -> Vec<K> {
    expected
        .keys()
        .chain(actual.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|key| expected.get(*key) != actual.get(*key))
        .cloned()
        .collect()
}
