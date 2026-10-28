use etas_std::{
    StdDecl, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind, StdType, TypeDecl, TypeDeclKind,
};

#[test]
fn schema_source_declaration_and_action_payload_share_one_canonical_type() {
    let registry = etas_std::standard_registry();
    let symbol = registry
        .lookup_qualified(&["std", "agent", "schema", "Schema"])
        .unwrap();
    let StdDecl::Type(declaration) = &symbol.decl else {
        panic!("Schema declaration")
    };
    assert_eq!(
        declaration.canonical_type,
        Some(StdType::Schema(Box::new(StdType::Var("T".into()))))
    );
    assert!(declaration.representation.is_none());
}

#[test]
fn canonical_types_reject_undeclared_parameters_and_nominal_storage_conflicts() {
    for declaration in [
        TypeDecl::generic("Broken", &[], TypeDeclKind::Support)
            .with_canonical_type(StdType::Schema(Box::new(StdType::Var("T".into())))),
        TypeDecl::generic("Broken", &["T"], TypeDeclKind::Struct)
            .with_canonical_type(StdType::Schema(Box::new(StdType::Var("T".into())))),
        TypeDecl::generic("Broken", &["T"], TypeDeclKind::Support)
            .with_representation(StdType::Var("T".into()))
            .with_canonical_type(StdType::Schema(Box::new(StdType::Var("T".into())))),
        TypeDecl::generic("Broken", &[], TypeDeclKind::Support)
            .with_canonical_type(StdType::Named("std.probe.Broken".into())),
    ] {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "probe"], "probe");
        builder.symbol(
            module,
            "Broken",
            StdSymbolKind::Type,
            StdDecl::Type(declaration),
            "invalid",
        );
        assert!(builder.try_finish().is_err());
    }
}
