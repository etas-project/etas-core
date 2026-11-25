//! Compiler and host execution contracts used to check the maintained std
//! sources. Ordinary type declarations, layouts, algorithms, aliases and
//! prelude exports are supplied by source, never by this catalog.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::StdSourceRole;
use crate::{
    IntrinsicDescriptor, StdDecl, StdPrimitiveType, StdRegistry, StdRegistryBuilder,
    StdRegistryVersion, StdSymbolKind,
};

#[derive(Deserialize, Serialize)]
struct ContractSet {
    schema: u32,
    compatibility: StdRegistryVersion,
    contracts: Vec<Contract>,
}

const PRIMITIVES: [StdPrimitiveType; 20] = [
    StdPrimitiveType::Bool,
    StdPrimitiveType::I8,
    StdPrimitiveType::I16,
    StdPrimitiveType::I32,
    StdPrimitiveType::I64,
    StdPrimitiveType::I128,
    StdPrimitiveType::ISize,
    StdPrimitiveType::U8,
    StdPrimitiveType::U16,
    StdPrimitiveType::U32,
    StdPrimitiveType::U64,
    StdPrimitiveType::U128,
    StdPrimitiveType::USize,
    StdPrimitiveType::F32,
    StdPrimitiveType::F64,
    StdPrimitiveType::Char,
    StdPrimitiveType::String,
    StdPrimitiveType::Bytes,
    StdPrimitiveType::Unit,
    StdPrimitiveType::Never,
];

#[derive(Deserialize, Serialize)]
struct Contract {
    path: Vec<String>,
    kind: StdSymbolKind,
    declaration: StdDecl,
    primitive: Option<IntrinsicDescriptor>,
    enum_owner: Option<Vec<String>>,
    #[serde(default)]
    flow_overloads: Vec<crate::FlowDecl>,
    #[serde(default)]
    action_overloads: Vec<crate::EffectActionDecl>,
    #[serde(default)]
    memory_place_result_argument: Option<usize>,
}

/// Content identity of the exact compiler/host contract set used by a build.
pub fn compiler_contract_identity() -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(include_bytes!("bootstrap-contracts.json"));
    let effects = StdSourceRole::EXECUTION_EFFECTS.map(|role| (role.path(), role.predeclaration()));
    let primitives =
        PRIMITIVES.map(|primitive| (primitive, StdSourceRole::Primitive(primitive).path()));
    let code_types = crate::CodeTypeRole::ALL.map(|role| {
        (
            role,
            role.source_path(),
            role.type_parameter_count(),
            role.has_read_only_erased_view(),
        )
    });
    let executions = super::StdExecutionRole::ALL.map(|role| {
        (
            role,
            role.role(),
            role.source_path(),
            role.associated_owners(),
            role.owner_type_parameter_count(),
            role.has_receiver(),
            role.is_public(),
            role.owner_is_public(),
        )
    });
    hash.update(
        &serde_json::to_vec(&(
            42,
            effects,
            primitives,
            code_types,
            executions,
            super::SOURCE_STORAGE_VIEW,
            super::SOURCE_STORAGE_BUILD,
        ))
        .expect("compiler role ABI"),
    );
    format!("blake3:{}", hash.finalize().to_hex())
}

/// Construct only the native execution query. Source HIR supplies ordinary
/// constructor identities before signature checking; the checked bundle owns
/// their complete declarations and all public exports.
pub fn compiler_contract_registry() -> StdRegistry {
    let contracts: ContractSet = serde_json::from_str(include_str!("bootstrap-contracts.json"))
        .expect("compiled bootstrap contracts");
    assert_eq!(contracts.schema, 3, "bootstrap contract schema");
    let mut builder = StdRegistryBuilder::new(contracts.compatibility);
    let mut modules = BTreeMap::new();
    let mut symbols = BTreeMap::new();
    for contract in contracts.contracts {
        let (name, parent) = contract.path.split_last().expect("contract path");
        let module = *modules.entry(parent.to_vec()).or_insert_with(|| {
            builder.module(&parent.iter().map(String::as_str).collect::<Vec<_>>(), "")
        });
        let symbol = if let Some(owner) = contract.enum_owner {
            let StdDecl::Flow(declaration) = contract.declaration else {
                panic!("enum ABI requires its constructor signature");
            };
            builder
                .enum_constructor(symbols[&owner], declaration, "")
                .expect("compiled enum ABI")
        } else {
            builder.symbol_with_intrinsic(
                module,
                name,
                contract.kind,
                contract.declaration,
                "",
                contract.primitive,
            )
        };
        for overload in contract.flow_overloads {
            builder
                .flow_overload(symbol, overload)
                .expect("compiled flow overload ABI");
        }
        for overload in contract.action_overloads {
            builder
                .action_overload(symbol, overload)
                .expect("compiled action overload ABI");
        }
        if let Some(argument) = contract.memory_place_result_argument {
            builder
                .memory_place_result(symbol, argument)
                .expect("compiled memory place ABI");
        }
        symbols.insert(contract.path, symbol);
    }
    for role in StdSourceRole::EXECUTION_EFFECTS {
        let path = role
            .path()
            .split('.')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let (name, parent) = path.split_last().unwrap();
        let module = *modules.entry(parent.to_vec()).or_insert_with(|| {
            builder.module(&parent.iter().map(String::as_str).collect::<Vec<_>>(), "")
        });
        let symbol = builder.symbol(module, name, role.symbol_kind(), role.predeclaration(), "");
        symbols.insert(path, symbol);
    }
    let core = builder.module(&["std", "core"], "");
    for primitive in PRIMITIVES {
        let path = StdSourceRole::Primitive(primitive).path();
        let name = path.rsplit('.').next().unwrap();
        let symbol = builder.symbol(
            core,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(crate::TypeDecl::primitive(name)),
            "",
        );
        builder.prelude(name, symbol);
    }
    // Value type names are completed by source_types::bind in the producer.
    // Full registry validation belongs after that source declaration phase.
    builder.finish_bootstrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_contracts_preserve_execution_abi_without_source_method_apis() {
        let kernel = compiler_contract_registry();
        let legacy = crate::standard_registry();
        assert_eq!(kernel.version(), legacy.version());
        assert_eq!(
            kernel.spec_impls().count(),
            0,
            "impls belong to checked source"
        );
        for contract in kernel.symbols() {
            if let Some(role) = StdSourceRole::from_path(&contract.qualified_path.join(".")) {
                if let Some((stable_id, core, requirement)) = role.effect_execution() {
                    let StdDecl::Effect(declaration) = &contract.decl else {
                        panic!("effect role");
                    };
                    assert_eq!(declaration.stable_id, Some(stable_id));
                    assert_eq!(declaration.core, core);
                    assert_eq!(declaration.runtime_requirement, Some(requirement));
                    assert!(declaration.extends.is_empty());
                    continue;
                }
                let StdDecl::Type(declaration) = &contract.decl else {
                    unreachable!()
                };
                assert_eq!(
                    declaration
                        .params
                        .iter()
                        .map(|p| p.kind)
                        .collect::<Vec<_>>(),
                    role.parameter_kinds()
                );
                assert_eq!(declaration.code_role, role.code_role());
                assert_eq!(contract.kind, role.symbol_kind());
                assert!(contract.intrinsic.is_none());
            } else {
                let native = legacy.lookup_qualified(&contract.qualified_path).unwrap();
                let mut native_declaration = native.decl.clone();
                if let (StdDecl::EffectAction(current), StdDecl::EffectAction(previous)) =
                    (&contract.decl, &mut native_declaration)
                {
                    for (current, previous) in
                        current.effect_args.iter().zip(&mut previous.effect_args)
                    {
                        if let (
                            crate::EffectActionArgKind::StaticResourcePath { ty: current },
                            crate::EffectActionArgKind::StaticResourcePath { ty: previous },
                        ) = (current, previous)
                            && current != previous
                        {
                            // Source contracts qualify resource identities that
                            // legacy signatures name in their owning module.
                            let owner = legacy
                                .lookup_type_dependency(&current.split('.').collect::<Vec<_>>())
                                .unwrap();
                            assert_eq!(previous.as_ref(), owner.qualified_path.last().unwrap());
                            *previous = current.clone();
                        }
                    }
                }
                if let (StdDecl::EffectAction(current), StdDecl::EffectAction(previous)) =
                    (&contract.decl, &native_declaration)
                    && current.owner == "Fs"
                {
                    // Source Fs uses the explicit limit/content payload ABI.
                    // Its canonical identity, authority selector and I/O failure
                    // mechanism still match the released host action. Actual
                    // source declarations validate the new payload signatures.
                    assert_eq!(current.stable_id, previous.stable_id);
                    assert_eq!(current.type_params, previous.type_params);
                    assert_eq!(current.effect_args, previous.effect_args);
                    assert_eq!(current.selector_param_names, previous.selector_param_names);
                    assert_eq!(current.runtime_requirement, previous.runtime_requirement);
                    assert_eq!(
                        current.default_handler_effects,
                        previous.default_handler_effects
                    );
                    assert_eq!(current.high_impact_ack, previous.high_impact_ack);
                    continue;
                }
                assert_eq!(contract.decl, native_declaration);
                assert_eq!(contract.intrinsic, native.intrinsic);
            }
        }
        assert!(kernel.lookup_qualified(&["std", "text", "chars"]).is_none());
        assert!(kernel.lookup_qualified(&["std", "text", "join"]).is_none());
        assert!(
            kernel
                .lookup_qualified(&["std", "task", "scheduler", "requeue"])
                .is_none()
        );
        assert!(
            kernel
                .lookup_qualified(&["std", "task", "scheduler", "park"])
                .is_none()
        );
        let execution_methods = kernel
            .symbols()
            .filter_map(|symbol| match &symbol.decl {
                StdDecl::Flow(flow) if flow.source_method.is_some() => {
                    Some(symbol.qualified_path.join("."))
                }
                _ => None,
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            execution_methods.is_empty(),
            "methods belong to checked source"
        );
        assert!(
            kernel
                .lookup_qualified(&["std", "text", "TextError"])
                .is_none()
        );
        assert_eq!(kernel.native_enum_references().count(), 0);
        assert!(kernel.lookup_qualified(&["std", "fs", "IOError"]).is_none());
        assert_eq!(kernel.native_opaque_references().count(), 0);
    }
}
