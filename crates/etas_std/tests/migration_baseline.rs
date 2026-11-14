use etas_std::migration::{StdBaselineImplementation, StdMigrationBaseline};
use etas_std::{
    FlowDecl, StdDecl, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind, standard_registry,
};

#[test]
fn complete_baseline_keeps_roles_overloads_exports_and_primitive_responsibilities() {
    let registry = standard_registry();
    let baseline = StdMigrationBaseline::from_registry(&registry);
    assert_eq!(baseline.symbols.len(), registry.symbols().count());
    assert_eq!(baseline.modules.len(), registry.modules().count());
    assert_eq!(baseline.prelude.len(), registry.prelude().symbols().count());
    assert_eq!(baseline.spec_impls.len(), registry.spec_impls().count());
    assert_eq!(baseline.modules["std.text"]["join"], "std.text.join");
    let result = &baseline.symbols[&baseline.prelude["Type:Result"]];
    assert_eq!(baseline.prelude["Type:Result"], "std.result.Result");
    assert_eq!(result.declaration["Type"]["kind"], "Enum");
    assert_eq!(
        result.declaration["Type"]["params"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    // Legacy Option/Result constructors are separate intrinsic-backed declarations.
    // Keep their actual identities rather than inventing a source enum layout.
    let ok = &baseline.symbols[&baseline.prelude["Value:Ok"]];
    assert_eq!(baseline.prelude["Value:Ok"], "std.result.Ok");
    assert_eq!(ok.kind, "Constructor");
    assert_eq!(ok.binding, Some(etas_std::intrinsic::pure::RESULT_OK));
    let index = &baseline.symbols[&baseline.prelude["Type:Index"]];
    assert_eq!(baseline.prelude["Type:Index"], "std.core.Index");
    assert_eq!(index.declaration["Type"]["kind"], "Spec");
    let task = &baseline.symbols[&baseline.prelude["Type:Task"]];
    assert_eq!(task.declaration["Type"]["runtime_role"], "Task");
    assert_eq!(
        baseline.symbols["std.task.map_concurrent"].declaration["Flow"]["public_effect_tail"],
        "E"
    );
    let join = &baseline.symbols["std.text.join"];
    assert!(
        matches!(&join.implementation, Some(StdBaselineImplementation::Source { source_digest, .. }) if source_digest.len()==64)
    );
    assert!(
        baseline.primitives[&join.binding.unwrap()]
            .accelerates
            .contains(&"std.text.join".to_owned())
    );
    let print = &baseline.symbols["std.io.println"];
    assert!(
        baseline.primitives[&print.binding.unwrap()]
            .required_by
            .contains(&"std.io.println".to_owned())
    );
    for symbol in registry.symbols() {
        let record = &baseline.symbols[&symbol.qualified_path.join(".")];
        let expected = match &symbol.decl {
            StdDecl::Flow(_) => registry.flow_declarations(symbol.id).count() - 1,
            StdDecl::EffectAction(_) => registry.action_declarations(symbol.id).count() - 1,
            _ => 0,
        };
        assert_eq!(record.overloads.len(), expected);
    }
    assert!(baseline.primitives.values().all(|primitive|
        !primitive.required_by.is_empty() || !primitive.accelerates.is_empty()));
    let wire = serde_json::to_vec(&baseline).unwrap();
    let restored: StdMigrationBaseline = serde_json::from_slice(&wire).unwrap();
    assert_eq!(restored, baseline);
    assert!(baseline.compare(&restored).matches());
    assert_eq!(
        wire,
        serde_json::to_vec(&StdMigrationBaseline::from_registry(&standard_registry())).unwrap()
    );
}

#[test]
fn canonical_inventory_ignores_registration_ordinals() {
    let build = |names: [&str; 2]| {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        for name in names {
            let module = builder.module(&["std", name], "migration probe");
            builder.symbol(
                module,
                "echo",
                StdSymbolKind::Flow,
                StdDecl::Flow(FlowDecl::pure("echo", &["string"], "string")),
                "echo",
            );
        }
        StdMigrationBaseline::from_registry(&builder.finish())
    };
    let first = build(["left", "right"]);
    let second = build(["right", "left"]);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
}

#[test]
fn migration_report_lists_module_coverage_and_source_contract_changes() {
    let expected = StdMigrationBaseline::from_registry(&standard_registry());
    let mut batch = expected.clone();
    batch
        .symbols
        .retain(|_, symbol| symbol.module == "std.text");
    batch.modules.retain(|module, _| module == "std.text");
    let join = batch.symbols.get_mut("std.text.join").unwrap();
    join.declaration["Flow"]["output"] = serde_json::json!({"Array": {"Primitive": "String"}});
    if let Some(StdBaselineImplementation::Source { source_digest, .. }) = &mut join.implementation
    {
        *source_digest = "rebuilt-source-body".into();
    }
    let report = expected.compare(&batch);
    assert!(
        report.modules["std.result"]
            .missing
            .contains(&"std.result.Result".to_owned())
    );
    assert!(report.modules["std.text"].missing.is_empty());
    assert!(
        report.modules["std.text"]
            .covered
            .contains(&"std.text.join".to_owned())
    );
    assert_eq!(
        report.modules["std.text"].contract_changes,
        ["std.text.join"]
    );
    assert_eq!(
        report.modules["std.text"].implementation_changes,
        ["std.text.join"]
    );
    assert!(!report.matches());
}
