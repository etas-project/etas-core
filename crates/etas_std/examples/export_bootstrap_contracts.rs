use etas_std::{StdDecl, StdRegistryVersion, StdSymbolKind};
use std::collections::BTreeSet;

fn main() {
    let registry = etas_std::standard_registry();
    // This is a maintainer export, never the std compilation path. Discover
    // native imports from the maintained source instead of another Rust API list.
    fn source_bindings(root: &std::path::Path, paths: &mut BTreeSet<String>) {
        let mut entries = std::fs::read_dir(root)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                source_bindings(&path, paths);
            } else if path.extension().is_some_and(|extension| extension == "es") {
                for line in std::fs::read_to_string(path).unwrap().lines() {
                    if let Some(binding) = line
                        .trim()
                        .strip_prefix("@std.binding(\"")
                        .and_then(|line| line.strip_suffix("\")"))
                    {
                        paths.insert(binding.to_owned());
                    }
                }
            }
        }
    }
    let mut paths = BTreeSet::new();
    source_bindings(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("library"),
        &mut paths,
    );
    paths.extend(
        registry
            .symbols()
            .filter(|symbol| {
                symbol.kind == StdSymbolKind::EffectAction
                    || symbol.kind == StdSymbolKind::Requirement
            })
            .map(|symbol| symbol.qualified_path.join(".")),
    );
    paths.retain(|path| etas_std::StdSourceRole::from_path(path).is_none());
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
        "flow_overloads": registry.flow_declarations(symbol.id).skip(1).collect::<Vec<_>>(),
        "action_overloads": registry.action_declarations(symbol.id).skip(1).collect::<Vec<_>>(),
        "memory_place_result_argument": registry.memory_place_result_argument(symbol.id),
    })).collect::<Vec<_>>();
    let types = registry
        .symbols()
        .filter(|symbol| matches!(symbol.decl, StdDecl::Type(_)))
        .map(|symbol| (symbol.qualified_path.join("."), symbol))
        .collect::<std::collections::BTreeMap<_, _>>();
    fn collect_names(value: &serde_json::Value, names: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::Object(fields) => {
                if let Some(serde_json::Value::String(name)) = fields.get("Named") {
                    names.insert(name.clone());
                }
                if let Some(name) = fields
                    .get("NamedApplied")
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
                {
                    names.insert(name.into());
                }
                for field in fields.values() {
                    collect_names(field, names);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    collect_names(value, names);
                }
            }
            _ => {}
        }
    }
    let mut imported = BTreeSet::new();
    let mut pending = contracts
        .iter()
        .map(|contract| {
            (
                contract["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join("."),
                contract.clone(),
            )
        })
        .collect::<Vec<_>>();
    while let Some((owner, value)) = pending.pop() {
        let mut names = BTreeSet::new();
        collect_names(&value, &mut names);
        for name in names {
            let mut scope = owner.split('.').collect::<Vec<_>>();
            scope.pop();
            let scoped = format!("{}.{}", scope.join("."), name);
            let target = types
                .get(&name)
                .or_else(|| types.get(&scoped))
                .copied()
                .or_else(|| {
                    registry
                        .lookup_prelude_in_namespace(&name, etas_std::StdNamespace::Type)
                        .and_then(|symbol| registry.symbol(symbol.id))
                })
                .or_else(|| {
                    let mut candidates =
                        types.values().copied().filter(|symbol| symbol.name == name);
                    let matched = candidates.next()?;
                    candidates.next().is_none().then_some(matched)
                });
            let Some(target) = target else {
                continue;
            };
            let path = target.qualified_path.join(".");
            if etas_std::StdSourceRole::from_path(&path).is_some() || !imported.insert(path.clone())
            {
                continue;
            }
            pending.push((path, serde_json::to_value(&target.decl).unwrap()));
            for constructor in registry
                .symbols()
                .filter(|symbol| symbol.enum_owner == Some(target.id))
            {
                pending.push((
                    constructor.qualified_path.join("."),
                    serde_json::to_value(&constructor.decl).unwrap(),
                ));
            }
        }
    }
    let native_type_references = imported
        .into_iter()
        .map(|path| {
            let symbol = types[&path];
            let StdDecl::Type(declaration) = &symbol.decl else {
                unreachable!()
            };
            serde_json::json!({"path": symbol.qualified_path,
            "parameters": declaration.params.iter().map(|p| p.kind).collect::<Vec<_>>(),
            "is_enum": declaration.kind == etas_std::TypeDeclKind::Enum,
            "opaque": declaration.kind != etas_std::TypeDeclKind::Enum && declaration.representation.is_none()})
        })
        .collect::<Vec<_>>();
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
    let mut native_record_references = vec![serde_json::json!({
        "path": ["std", "fs", "FsStat"],
        "fields": [
            {"name": "is_file", "ty": {"Primitive": "Bool"}},
            {"name": "is_dir", "ty": {"Primitive": "Bool"}},
            {"name": "len", "ty": {"Primitive": "U64"}},
        ],
    })];
    // Compiler report payload layouts are ABI constraints, not public declarations.
    for symbol in registry.symbols() {
        if !symbol
            .qualified_path
            .starts_with(&["std".into(), "compiler".into()])
        {
            continue;
        }
        if let StdDecl::Type(ty) = &symbol.decl
            && ty.code_role.is_none()
            && let Some(etas_std::StdType::Record(fields)) = &ty.representation
        {
            native_record_references.push(serde_json::json!({
                "path": symbol.qualified_path,
                "fields": fields,
            }));
        }
    }
    let native_projection_references = registry
        .symbols()
        .filter_map(|symbol| {
            let StdDecl::Type(ty) = &symbol.decl else {
                return None;
            };
            ((ty.code_role.is_some() || ty.runtime_role.is_some())
                && !ty.projected_fields.is_empty())
            .then(|| {
                serde_json::json!({
                    "path": symbol.qualified_path, "fields": ty.projected_fields,
                })
            })
        })
        .collect::<Vec<_>>();
    let mut native_enum_references = vec![serde_json::json!({"path": ["std", "text", "TextError"],
            "required_unit_variants": ["ParseFailure", "NumericOverflow"], "required_payload_variants": {}})];
    // The existing host reducers/decoders require these concrete primitive
    // payloads. Generic and nominal enum payloads stay in the source type graph.
    for symbol in registry.symbols() {
        let StdDecl::Type(declaration) = &symbol.decl else {
            continue;
        };
        if declaration.kind != etas_std::TypeDeclKind::Enum
            || !declaration.params.is_empty()
            || symbol.qualified_path == ["std", "text", "TextError"]
        {
            continue;
        }
        let constructors = registry.enum_constructors(symbol.id).collect::<Vec<_>>();
        let mut unit = Vec::new();
        let mut payload = std::collections::BTreeMap::new();
        let mut primitive_only = !constructors.is_empty();
        for constructor in constructors {
            let StdDecl::Flow(flow) = &constructor.decl else {
                unreachable!()
            };
            if flow.params.is_empty() {
                unit.push(flow.name.clone());
            } else {
                let fields = flow
                    .params
                    .iter()
                    .map(|ty| match ty {
                        etas_std::StdType::Primitive(primitive) => Some(*primitive),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>();
                if let Some(fields) = fields {
                    payload.insert(flow.name.clone(), fields);
                } else {
                    primitive_only = false;
                }
            }
        }
        if primitive_only {
            native_enum_references.push(serde_json::json!({"path": symbol.qualified_path,
                "required_unit_variants": unit, "required_payload_variants": payload}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": 2, "compatibility": StdRegistryVersion::phase1(),
            "contracts": contracts, "prelude": prelude, "aliases": aliases,
            "native_type_references": native_type_references,
            "native_enum_references": native_enum_references,
            "native_opaque_references": [
                {"path": ["std", "fs", "IOError"]},
                {"path": ["std", "io", "IOError"]},
                {"path": ["std", "runtime", "time", "Duration"]},
            ],
            "native_record_references": native_record_references,
            "native_projection_references": native_projection_references,
            "implementations": [],
        }))
        .unwrap()
    );
}
