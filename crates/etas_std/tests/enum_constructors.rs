use etas_std::{
    FlowDecl, StdDecl, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind, TypeDecl,
    TypeDeclKind,
};

#[test]
fn memory_match_export_preserves_canonical_constructor_identity() {
    let registry = etas_std::standard_registry();
    let owner = registry
        .lookup_qualified(&["std", "memory", "WriteCondition"])
        .unwrap();
    let constructor = registry.enum_constructor(owner.id, "Match").unwrap();
    let exported = registry
        .lookup_qualified(&["std", "memory", "Match"])
        .unwrap();
    assert_eq!(exported.id, constructor.id);
    assert_eq!(exported.enum_owner, Some(owner.id));
    assert_eq!(
        constructor.qualified_path.join("."),
        "std.memory.WriteCondition.Match"
    );
    assert_eq!(registry.enum_constructors(owner.id).count(), 4);
    assert!(
        registry
            .module_exports(owner.module)
            .any(|(name, symbol)| name == "Match" && symbol.id == constructor.id)
    );
    let StdDecl::Flow(signature) = &constructor.decl else {
        panic!("constructor signature")
    };
    assert_eq!(
        signature.params,
        vec![etas_std::StdType::Named("MemoryVersion".into())]
    );
    assert_eq!(
        signature.output,
        etas_std::StdType::Named("std.memory.WriteCondition".into())
    );
}

#[test]
fn registry_re_exports_reject_missing_targets_and_path_collisions() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "test"], "test");
    let owner = builder.symbol(
        module,
        "Outcome",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Outcome", &[], TypeDeclKind::Enum)),
        "test",
    );
    let constructor = builder
        .enum_constructor(
            owner,
            FlowDecl::pure("Done", &[], "std.test.Outcome"),
            "test",
        )
        .unwrap();
    assert!(
        builder
            .re_export(module, "Done", etas_std::StdSymbolId(u32::MAX))
            .is_err()
    );
    assert!(builder.re_export(module, "Outcome", constructor).is_err());
    assert!(builder.re_export(module, "", constructor).is_err());
    assert!(builder.re_export(module, "Bad.Path", constructor).is_err());
    builder.re_export(module, "Done", constructor).unwrap();
    assert!(builder.re_export(module, "Done", constructor).is_err());
    builder.symbol(
        module,
        "Done",
        StdSymbolKind::Value,
        StdDecl::Value(etas_std::ValueDecl::new("Done", "std.test.Outcome")),
        "test",
    );
    assert!(
        builder
            .try_finish()
            .unwrap_err()
            .reason
            .contains("re-export")
    );
}

#[test]
fn storage_outcomes_have_distinct_owned_constructors_and_closed_confirmation() {
    let registry = etas_std::standard_registry();
    let write = registry
        .lookup_qualified(&["std", "memory", "WriteOutcome"])
        .unwrap();
    let confirmed = registry
        .lookup_qualified(&["std", "memory", "ConfirmedOutcome"])
        .unwrap();
    let reconcile = registry
        .lookup_qualified(&["std", "memory", "ReconcileResult"])
        .unwrap();
    let write_committed = registry.enum_constructor(write.id, "Committed").unwrap();
    let confirmed_committed = registry
        .enum_constructor(confirmed.id, "Committed")
        .unwrap();
    assert_ne!(write_committed.id, confirmed_committed.id);
    assert_ne!(
        write_committed.qualified_path,
        confirmed_committed.qualified_path
    );
    assert!(registry.enum_constructor(write.id, "Unknown").is_some());
    assert!(registry.enum_constructor(confirmed.id, "Unknown").is_none());
    assert!(registry.enum_constructor(reconcile.id, "Found").is_some());
    assert!(
        registry
            .lookup_qualified(&["std", "memory", "Committed"])
            .is_none()
    );
    assert!(registry.lookup_prelude("Committed").is_none());
    let StdDecl::Flow(found) = &registry
        .enum_constructor(reconcile.id, "Found")
        .unwrap()
        .decl
    else {
        panic!("constructor");
    };
    assert_eq!(
        found.params,
        vec![etas_std::StdType::NamedApplied {
            name: "std.memory.ConfirmedOutcome".into(),
            args: vec![
                etas_std::StdType::Var("R".into()),
                etas_std::StdType::Var("N".into())
            ],
        }]
    );
}

#[test]
fn registry_rejects_constructor_with_different_owner_result_or_generics() {
    for output in ["std.test.Other<T>", "std.test.Outcome<string>"] {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "test"], "test");
        let owner = builder.symbol(
            module,
            "Outcome",
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic("Outcome", &["T"], TypeDeclKind::Enum)),
            "test",
        );
        builder
            .enum_constructor(
                owner,
                FlowDecl::with_type_params_actions(
                    "Value",
                    &[etas_std::StdGenericParam::new("T")],
                    &["T"],
                    output,
                    &[],
                    &[],
                ),
                "test",
            )
            .unwrap();
        assert!(
            builder
                .try_finish()
                .unwrap_err()
                .reason
                .contains("canonical owner")
        );
    }
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "test"], "test");
    let owner = builder.symbol(
        module,
        "Outcome",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Outcome", &["T"], TypeDeclKind::Enum)),
        "test",
    );
    builder
        .enum_constructor(
            owner,
            FlowDecl::pure("Value", &[], "std.test.Outcome"),
            "test",
        )
        .unwrap();
    assert!(
        builder
            .try_finish()
            .unwrap_err()
            .reason
            .contains("owner parameters")
    );
}

#[test]
fn registry_rejects_non_enum_constructor_owner() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    let module = builder.module(&["std", "test"], "test");
    let owner = builder.symbol(
        module,
        "Opaque",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Opaque", &[], TypeDeclKind::Support)),
        "test",
    );
    assert!(
        builder
            .enum_constructor(
                owner,
                FlowDecl::pure("Value", &[], "std.test.Opaque"),
                "test"
            )
            .is_err()
    );
}
