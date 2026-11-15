//! Compiler and host ABI needed to check the maintained std sources. This
//! catalog has no source algorithms, bodies, docs, or legacy registry identities.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::StdSourceRole;
use crate::{
    IntrinsicDescriptor, StdDecl, StdImplFact, StdRegistry, StdRegistryBuilder, StdRegistryVersion,
    StdSymbolKind,
};

#[derive(Deserialize, Serialize)]
struct ContractSet {
    schema: u32,
    compatibility: StdRegistryVersion,
    contracts: Vec<Contract>,
    native_enum_references: Vec<crate::StdNativeEnumReference>,
    prelude: Vec<(String, Vec<String>)>,
    aliases: Vec<(Vec<String>, Vec<String>)>,
    implementations: Vec<StdImplFact>,
}

#[derive(Deserialize, Serialize)]
struct Contract {
    path: Vec<String>,
    kind: StdSymbolKind,
    declaration: StdDecl,
    primitive: Option<IntrinsicDescriptor>,
    enum_owner: Option<Vec<String>>,
}

/// Content identity of the exact compiler/host contract set used by a build.
pub fn compiler_contract_identity() -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(include_bytes!("bootstrap-contracts.json"));
    let roles = StdSourceRole::ALL.map(|role| {
        (
            role,
            role.path(),
            role.parameter_kinds(),
            role.runtime_role(),
        )
    });
    hash.update(&serde_json::to_vec(&(1, roles)).expect("compiler role ABI"));
    format!("blake3:{}", hash.finalize().to_hex())
}

/// Construct only the explicit compiler/host ABI. Public std interfaces are
/// supplied by checked Etas declarations when the bundle query view is built.
pub fn compiler_contract_registry() -> StdRegistry {
    let contracts: ContractSet = serde_json::from_str(include_str!("bootstrap-contracts.json"))
        .expect("compiled bootstrap contracts");
    assert_eq!(contracts.schema, 1, "bootstrap contract schema");
    let mut builder = StdRegistryBuilder::new(contracts.compatibility);
    for reference in contracts.native_enum_references {
        builder.native_enum_reference(reference);
    }
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
        symbols.insert(contract.path, symbol);
    }
    // These are compiler type constructors, not a second public std API. The
    // checked source supplies their actual parameter names and declarations.
    for role in StdSourceRole::ALL {
        let path = role
            .path()
            .split('.')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let (name, parent) = path.split_last().unwrap();
        let module = *modules.entry(parent.to_vec()).or_insert_with(|| {
            builder.module(&parent.iter().map(String::as_str).collect::<Vec<_>>(), "")
        });
        let symbol = builder.symbol(
            module,
            name,
            role.symbol_kind(),
            StdDecl::Type(role.predeclaration()),
            "",
        );
        symbols.insert(path, symbol);
    }
    for (alias, target) in contracts.aliases {
        let (name, parent) = alias.split_last().expect("contract alias");
        let module = *modules.entry(parent.to_vec()).or_insert_with(|| {
            builder.module(&parent.iter().map(String::as_str).collect::<Vec<_>>(), "")
        });
        builder
            .re_export(module, name, symbols[&target])
            .expect("compiled ABI alias");
    }
    for (name, target) in contracts.prelude {
        builder.prelude(&name, symbols[&target]);
    }
    for implementation in contracts.implementations {
        builder.spec_impl(implementation);
    }
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_contracts_preserve_native_abi_without_std_algorithms() {
        let kernel = compiler_contract_registry();
        let legacy = crate::standard_registry();
        assert_eq!(kernel.version(), legacy.version());
        assert_eq!(
            kernel.spec_impls().count(),
            0,
            "impls belong to checked source"
        );
        for contract in kernel.symbols() {
            let native = legacy.lookup_qualified(&contract.qualified_path).unwrap();
            if let Some(role) = StdSourceRole::from_path(&contract.qualified_path.join(".")) {
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
                assert_eq!(contract.kind, role.symbol_kind());
                assert!(contract.intrinsic.is_none());
            } else {
                assert_eq!(contract.decl, native.decl);
                assert_eq!(contract.intrinsic, native.intrinsic);
            }
        }
        assert!(kernel.lookup_qualified(&["std", "text", "chars"]).is_some());
        assert!(kernel.lookup_qualified(&["std", "text", "join"]).is_none());
        assert!(
            kernel
                .lookup_qualified(&["std", "text", "TextError"])
                .is_none()
        );
        assert_eq!(
            kernel
                .native_enum_reference("std.text.TextError")
                .unwrap()
                .required_unit_variants,
            ["ParseFailure", "NumericOverflow"]
        );
    }
}
