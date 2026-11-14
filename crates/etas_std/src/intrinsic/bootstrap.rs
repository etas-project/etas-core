//! Compiler and host ABI needed to check the maintained std sources. This
//! catalog has no source algorithms, bodies, docs, or legacy registry identities.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    IntrinsicDescriptor, StdDecl, StdImplFact, StdRegistry, StdRegistryBuilder, StdRegistryVersion,
    StdSymbolKind,
};

#[derive(Deserialize, Serialize)]
struct ContractSet {
    schema: u32,
    compatibility: StdRegistryVersion,
    contracts: Vec<Contract>,
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
    format!(
        "blake3:{}",
        blake3::hash(include_bytes!("bootstrap-contracts.json")).to_hex()
    )
}

/// Construct only the explicit compiler/host ABI. Public std interfaces are
/// supplied by checked Etas declarations when the bundle query view is built.
pub fn compiler_contract_registry() -> StdRegistry {
    let contracts: ContractSet = serde_json::from_str(include_str!("bootstrap-contracts.json"))
        .expect("compiled bootstrap contracts");
    assert_eq!(contracts.schema, 1, "bootstrap contract schema");
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
        symbols.insert(contract.path, symbol);
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
            assert_eq!(contract.decl, native.decl);
            assert_eq!(contract.intrinsic, native.intrinsic);
        }
        assert!(kernel.lookup_qualified(&["std", "text", "chars"]).is_some());
        assert!(kernel.lookup_qualified(&["std", "text", "join"]).is_none());
    }
}
