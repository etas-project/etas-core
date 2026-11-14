use etas_code::{CallableId, DefinitionKey, DefinitionKind, PackageKey};
use etas_std::{FlowDecl, StdRegistryBuilder, StdRegistryImage, standard_registry};

#[test]
fn portable_registry_preserves_native_ids_roles_and_source_callable_identity() {
    let native = standard_registry().native_bootstrap();
    let module = native
        .modules()
        .find(|module| module.path == ["std", "text"])
        .unwrap()
        .id;
    let expected = native
        .symbols()
        .map(|symbol| (symbol.id, symbol.qualified_path.clone()))
        .collect::<Vec<_>>();
    let key = DefinitionKey::new(
        PackageKey::from_name_and_version("std", "0.2.4").unwrap(),
        vec!["std".into(), "text".into()],
        vec!["from_bundle".into()],
        DefinitionKind::Flow,
    )
    .unwrap();
    let mut builder = StdRegistryBuilder::from_registry(native);
    let id = builder.source_flow(
        module,
        "from_bundle",
        FlowDecl::pure("from_bundle", &["string"], "string"),
        CallableId::declaration(key.clone()).unwrap(),
        "Source-defined callable",
    );
    builder.prelude("from_bundle", id);
    let registry = builder.finish();
    let encoded = serde_json::to_vec(&StdRegistryImage::capture(&registry).unwrap()).unwrap();
    let image: StdRegistryImage = serde_json::from_slice(&encoded).unwrap();
    let loaded = image.into_registry().unwrap();
    for (id, path) in expected {
        assert_eq!(loaded.symbol(id).unwrap().qualified_path, path);
    }
    assert_eq!(
        loaded
            .lookup_qualified(&["std", "text", "from_bundle"])
            .unwrap()
            .id,
        id
    );
    assert_eq!(loaded.lookup_prelude("from_bundle").unwrap().id, id);
    assert_eq!(
        loaded.body_binding(id).unwrap().callable.as_declaration(),
        Some(&key)
    );
    assert_eq!(
        serde_json::to_vec(&StdRegistryImage::capture(&loaded).unwrap()).unwrap(),
        encoded
    );
    let bootstrap = loaded.native_bootstrap();
    bootstrap.validate().unwrap();
    assert!(
        bootstrap
            .lookup_qualified(&["std", "text", "from_bundle"])
            .is_none()
    );
    assert!(bootstrap.lookup_prelude("from_bundle").is_none());
}
