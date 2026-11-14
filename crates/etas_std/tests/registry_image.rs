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

#[test]
fn source_flow_replaces_native_public_slot_and_keeps_prelude_identity() {
    let native = standard_registry().native_bootstrap();
    let original = native
        .lookup_qualified(&["std", "text", "trim"])
        .unwrap()
        .clone();
    let mut builder = StdRegistryBuilder::from_registry(native);
    builder.prelude("trim", original.id);
    let key = DefinitionKey::new(
        PackageKey::from_name_and_version("std", "0.2.5").unwrap(),
        vec!["std".into(), "text".into()],
        vec!["trim".into()],
        DefinitionKind::Flow,
    )
    .unwrap();
    let etas_std::StdDecl::Flow(declaration) = original.decl else {
        panic!("native flow");
    };
    let id = builder.source_flow(
        original.module,
        "trim",
        declaration,
        CallableId::declaration(key.clone()).unwrap(),
        "",
    );
    let registry = builder.try_finish().unwrap();
    assert_eq!(id, original.id);
    assert_eq!(registry.lookup_prelude("trim").unwrap().id, id);
    assert!(registry.symbol(id).unwrap().intrinsic.is_none());
    let image = StdRegistryImage::capture(&registry).unwrap();
    let restored = image.into_registry().unwrap();
    assert_eq!(
        restored.body_binding(id).unwrap().callable.as_declaration(),
        Some(&key)
    );
    assert!(matches!(
        restored.symbol(id).unwrap().implementation,
        Some(etas_std::StdImplementationRef::Artifact { .. })
    ));
}

#[test]
fn source_enum_preserves_canonical_owners_and_named_fields_across_images() {
    use etas_std::{StdDecl, StdGenericParam, StdSymbolKind, StdType, TypeDecl, TypeDeclKind};
    let mut builder = StdRegistryBuilder::from_registry(standard_registry().native_bootstrap());
    let module = builder.module(&["std", "probe"], "");
    let key = DefinitionKey::new(
        PackageKey::from_name_and_version("std", "0.2.6").unwrap(),
        vec!["std".into(), "probe".into()],
        vec!["Reply".into()],
        DefinitionKind::Type,
    )
    .unwrap();
    let owner = builder.symbol(
        module,
        "Reply",
        StdSymbolKind::Type,
        StdDecl::Type(TypeDecl::generic("Reply", &["T"], TypeDeclKind::Enum)),
        "",
    );
    builder.bind_declaration(owner, key.clone());
    let mut constructor = FlowDecl::pure("Item", &[], "unit");
    constructor.type_params = vec![StdGenericParam::new("T")];
    constructor.params = vec![StdType::Var("T".into())];
    constructor.output = StdType::NamedApplied {
        name: "std.probe.Reply".into(),
        args: vec![StdType::Var("T".into())],
    };
    let variant = builder.enum_constructor(owner, constructor, "").unwrap();
    let variant_key = DefinitionKey::new(
        key.package(),
        key.module().to_vec(),
        vec!["Reply".into(), "Item".into()],
        DefinitionKind::EnumVariant,
    )
    .unwrap();
    builder.bind_declaration(variant, variant_key.clone());
    builder.named_enum_fields(variant, vec!["value".into()]);
    let registry = builder.try_finish().unwrap();
    let bytes = serde_json::to_vec(&StdRegistryImage::capture(&registry).unwrap()).unwrap();
    let image: StdRegistryImage = serde_json::from_slice(&bytes).unwrap();
    let loaded = image.into_registry().unwrap();
    assert_eq!(loaded.declaration_key(owner), Some(&key));
    assert_eq!(loaded.declaration_key(variant), Some(&variant_key));
    assert_eq!(
        loaded.enum_field_names(variant),
        Some(["value".into()].as_slice())
    );
    assert_eq!(loaded.enum_constructor(owner, "Item").unwrap().id, variant);
    let bootstrap = loaded.native_bootstrap();
    bootstrap.validate().unwrap();
    assert!(
        bootstrap
            .lookup_qualified(&["std", "probe", "Reply"])
            .is_none()
    );
    assert_eq!(
        serde_json::to_vec(&StdRegistryImage::capture(&loaded).unwrap()).unwrap(),
        bytes
    );
}
