use etas_std::{StdDecl, StdRegistryVersion, StdSymbolKind};
use std::collections::BTreeSet;

fn main() {
    let registry = etas_std::standard_registry();
    let mut paths = [
        "std.core.abort",
        "std.char.is_whitespace",
        "std.runtime.error.raise",
        "std.runtime.budget.Money",
        "std.runtime.time.Duration",
        "std.runtime.limits.Limit",
        "std.collections.array_get",
        "std.collections.array_push",
        "std.text.chars",
        "std.text.from_char",
        "std.text.parse_usize",
        "std.text.uppercase",
        "std.text.lowercase",
        "std.task.run",
        "std.task.Async",
        "std.task.actions.Async.yield",
        "std.task.actions.Async.await",
        "std.effects.Error",
        "std.effects.FileIO",
        "std.effects.Fs",
        "std.effects.actions.Fs.read",
        "std.fs.path",
        "std.fs.read_bytes",
        "std.fs.root",
        "std.fs.relative_name",
        "std.fs.read_bytes_limited",
        "std.fs.write_bytes",
        "std.fs.list",
        "std.fs.list_limited",
        "std.fs.stat",
        "std.fs.atomic_replace",
        "std.effects.actions.Fs.write",
        "std.effects.actions.Fs.list",
        "std.effects.actions.Fs.stat",
        "std.effects.actions.Fs.atomic_replace",
        "std.codec.text.utf8_encode",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    for symbol in registry.symbols() {
        if matches!(symbol.decl, StdDecl::Type(ref ty) if ty.kind == etas_std::TypeDeclKind::Primitive)
            || (symbol
                .qualified_path
                .starts_with(&["std".into(), "compiler".into()])
                && matches!(symbol.decl, StdDecl::Type(_)))
            || matches!(symbol.decl, StdDecl::Effect(ref effect) if effect.core)
            || symbol.kind == StdSymbolKind::Requirement
        {
            paths.insert(symbol.qualified_path.join("."));
        }
    }
    // Native enum payloads are part of the compiler ABI, including limits.
    let constructors = registry
        .symbols()
        .filter_map(|symbol| {
            let owner = registry.symbol(symbol.enum_owner?)?;
            paths
                .contains(&owner.qualified_path.join("."))
                .then(|| symbol.qualified_path.join("."))
        })
        .collect::<Vec<_>>();
    paths.extend(constructors);
    let selected = paths
        .iter()
        .map(|path| {
            registry
                .lookup_qualified(&path.split('.').collect::<Vec<_>>())
                .unwrap_or_else(|| panic!("missing {path}"))
        })
        .collect::<Vec<_>>();
    let mut ids = selected
        .iter()
        .map(|symbol| symbol.id)
        .collect::<BTreeSet<_>>();
    // Keep transitional prelude references to compiler roles, but never copy
    // their public declarations or constructor primitives into this catalog.
    ids.extend(etas_std::StdSourceRole::ALL.into_iter().map(|role| {
        registry
            .lookup_qualified(&role.path().split('.').collect::<Vec<_>>())
            .unwrap()
            .id
    }));
    let contracts = selected.into_iter().map(|symbol| serde_json::json!({
        "path": symbol.qualified_path,
        "kind": symbol.kind,
        "declaration": symbol.decl,
        "primitive": symbol.intrinsic,
        "enum_owner": symbol.enum_owner.map(|id| registry.symbol(id).unwrap().qualified_path.clone()),
    })).collect::<Vec<_>>();
    let prelude = registry
        .prelude()
        .symbols()
        .filter(|symbol| ids.contains(&symbol.id))
        .map(|symbol| {
            (
                symbol.name.clone(),
                registry.symbol(symbol.id).unwrap().qualified_path.clone(),
            )
        })
        .collect::<Vec<_>>();
    let aliases = registry
        .modules()
        .flat_map(|module| {
            registry
                .module_exports(module.id)
                .filter(|(_, symbol)| ids.contains(&symbol.id))
                .filter_map(move |(name, symbol)| {
                    let mut alias = module.path.clone();
                    alias.push(name.into());
                    (alias != symbol.qualified_path)
                        .then_some((alias, symbol.qualified_path.clone()))
                })
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": 1, "compatibility": StdRegistryVersion::phase1(),
            "contracts": contracts, "prelude": prelude, "aliases": aliases,
            "native_enum_references": [{
                "path": ["std", "text", "TextError"],
                "required_unit_variants": ["ParseFailure", "NumericOverflow"],
                "required_payload_variants": {},
            }, {
                "path": ["std", "fs", "ReadLimitError"],
                "required_unit_variants": [],
                "required_payload_variants": {"LimitExceeded": ["USize"]},
            }, {
                "path": ["std", "fs", "ListLimitError"],
                "required_unit_variants": [],
                "required_payload_variants": {"LimitExceeded": ["USize", "USize"]},
            }],
            "native_opaque_references": [{"path": ["std", "fs", "IOError"]}],
            "native_record_references": [{
                "path": ["std", "fs", "FsStat"],
                "fields": [
                    {"name": "is_file", "ty": {"Primitive": "Bool"}},
                    {"name": "is_dir", "ty": {"Primitive": "Bool"}},
                    {"name": "len", "ty": {"Primitive": "U64"}},
                ],
            }],
            "implementations": [],
        }))
        .unwrap()
    );
}
