use etas_std::{
    StdDecl, StdNamespace, StdPrelude, StdRegistryBuilder, StdRegistryVersion, StdSymbolId,
    StdSymbolKind, TypeDecl, TypeDeclKind, standard_registry,
};

#[test]
fn prelude_preserves_the_secret_type_and_effect() {
    let registry = standard_registry();
    let secrets = registry
        .prelude()
        .symbols()
        .filter(|binding| binding.name == "Secret")
        .map(|binding| {
            registry
                .symbol(binding.id)
                .expect("registered prelude symbol")
        })
        .collect::<Vec<_>>();
    assert_eq!(secrets.len(), 2, "{secrets:?}");
    assert!(
        secrets
            .iter()
            .any(|symbol| symbol.kind == StdSymbolKind::Type)
    );
    assert!(
        secrets
            .iter()
            .any(|symbol| symbol.kind == StdSymbolKind::Effect)
    );
    assert!(registry.lookup_prelude("Secret").is_none());
    assert_eq!(
        registry
            .lookup_prelude_in_namespace("Secret", StdNamespace::Type)
            .unwrap()
            .qualified_path,
        ["std", "security", "trust", "Secret"],
    );
    assert_eq!(
        registry
            .lookup_prelude_in_namespace("Secret", StdNamespace::Effect)
            .unwrap()
            .qualified_path,
        ["std", "effects", "Secret"],
    );
}

#[test]
fn ambiguous_prelude_lookup_never_uses_registration_order() {
    for ids in [
        [StdSymbolId(3), StdSymbolId(7)],
        [StdSymbolId(7), StdSymbolId(3)],
    ] {
        let mut prelude = StdPrelude::default();
        for id in ids {
            prelude.insert("Shared", id);
        }
        assert!(prelude.lookup("Shared").is_none());
        assert_eq!(prelude.candidates("Shared").count(), 2);
        prelude.insert("Shared", ids[0]);
        assert_eq!(prelude.candidates("Shared").count(), 2);
    }
}

#[test]
fn registry_rejects_conflicting_bindings_in_one_prelude_namespace() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    for module in ["first", "second"] {
        let module = builder.module(&["std", module], "test");
        let symbol = builder.symbol(
            module,
            "Shared",
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic("Shared", &[], TypeDeclKind::Struct)),
            "test",
        );
        builder.prelude("Shared", symbol);
    }
    let error = builder.try_finish().expect_err("ambiguous type prelude");
    assert_eq!(error.symbol, "Shared");
    assert!(error.reason.contains("same namespace"), "{error}");
}

#[test]
fn registry_rejects_unregistered_prelude_symbols() {
    let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
    builder.prelude("Missing", StdSymbolId(27));
    let error = builder.try_finish().expect_err("missing prelude target");
    assert!(error.reason.contains("unregistered symbol"), "{error}");
}
