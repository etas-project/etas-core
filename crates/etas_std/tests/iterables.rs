use etas_std::{
    StdDecl, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind, TypeDecl, TypeDeclKind,
    standard_registry,
};

#[test]
fn sequential_support_collections_declare_their_element_parameter() {
    let registry = standard_registry();
    for name in ["Deque", "Queue", "Stack"] {
        let symbol = registry
            .lookup_qualified(&["std", "collections", name])
            .unwrap();
        let StdDecl::Type(declaration) = &symbol.decl else {
            panic!("type declaration")
        };
        assert_eq!(declaration.iterable_element_param, Some(0));
        assert_eq!(declaration.params[0].name, "T");
    }
}

#[test]
fn iterable_declaration_rejects_missing_parameters_and_non_support_types() {
    for (kind, index) in [
        (TypeDeclKind::Support, 1),
        (TypeDeclKind::Spec, 0),
        (TypeDeclKind::Wrapper, 0),
    ] {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "probe"], "probe");
        builder.symbol(
            module,
            "Items",
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic("Items", &["T"], kind).iterable_elements(index)),
            "probe",
        );
        let error = builder.try_finish().unwrap_err();
        assert!(
            error
                .reason
                .contains("iterable element must reference a declared support type parameter"),
            "{error}"
        );
    }
}
