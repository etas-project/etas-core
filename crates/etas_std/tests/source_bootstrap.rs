use etas_std::{StdImplementationRef, standard_registry};

#[test]
fn native_bootstrap_keeps_native_abi_and_prelude_without_legacy_source_declarations() {
    let legacy = standard_registry();
    let join = legacy.lookup_qualified(&["std", "text", "join"]).unwrap();
    let count = legacy
        .lookup_qualified(&["std", "text", "char_count"])
        .unwrap();
    let bootstrap = legacy.native_bootstrap();
    assert!(
        bootstrap
            .lookup_qualified(&["std", "text", "join"])
            .is_none()
    );
    assert!(bootstrap.symbol(join.id).is_none());
    assert_eq!(
        bootstrap
            .lookup_qualified(&["std", "text", "char_count"])
            .unwrap()
            .id,
        count.id
    );
    assert!(bootstrap.symbols().all(|symbol| !matches!(
        symbol.implementation,
        Some(StdImplementationRef::Source { .. })
    )));
    assert!(
        bootstrap
            .modules()
            .flat_map(|module| bootstrap.module_exports(module.id))
            .all(|(_, symbol)| !matches!(
                symbol.implementation,
                Some(StdImplementationRef::Source { .. })
            ))
    );
    assert!(
        bootstrap
            .prelude()
            .symbols()
            .all(|binding| bootstrap.symbol(binding.id).is_some())
    );
    bootstrap.validate().unwrap();
    assert!(legacy.symbol(join.id).is_some());
}
