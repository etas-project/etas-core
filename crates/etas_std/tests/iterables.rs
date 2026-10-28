use etas_std::{
    StdDecl, StdIterableItem, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind, TypeDecl,
    TypeDeclKind, standard_registry,
};

#[test]
fn sequential_support_collections_declare_their_element_parameter() {
    let registry = standard_registry();
    for name in ["Deque", "Queue", "Stack", "OrderedSet"] {
        let symbol = registry
            .lookup_qualified(&["std", "collections", name])
            .unwrap();
        let StdDecl::Type(declaration) = &symbol.decl else {
            panic!("type declaration")
        };
        assert_eq!(
            declaration.iterable_item,
            Some(StdIterableItem::Element { parameter: 0 })
        );
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
                .contains("iterable item must reference declared support type parameters"),
            "{error}"
        );
    }
}

#[test]
fn entry_collections_declare_key_and_value_parameter_order() {
    let registry = standard_registry();
    for (name, key, value) in [("OrderedMap", 0, 1), ("PriorityQueue", 1, 0)] {
        let symbol = registry
            .lookup_qualified(&["std", "collections", name])
            .unwrap();
        let StdDecl::Type(declaration) = &symbol.decl else {
            panic!("type declaration")
        };
        assert_eq!(
            declaration.iterable_item,
            Some(StdIterableItem::Entry { key, value })
        );
    }
}

#[test]
fn entry_iterable_rejects_out_of_range_key_or_value() {
    for (key, value) in [(2, 0), (0, 2)] {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "probe"], "probe");
        builder.symbol(
            module,
            "Entries",
            StdSymbolKind::Type,
            StdDecl::Type(
                TypeDecl::generic("Entries", &["K", "V"], TypeDeclKind::Support)
                    .iterable_entries(key, value),
            ),
            "probe",
        );
        assert!(
            builder
                .try_finish()
                .unwrap_err()
                .reason
                .contains("iterable item")
        );
    }
}
