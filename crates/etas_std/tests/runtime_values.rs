use etas_std::{
    StdDecl, StdRegistryBuilder, StdRegistryVersion, StdSymbolKind, StdType, StdValueRole,
    TypeDecl, TypeDeclKind, ValueDecl, standard_registry,
};

#[test]
fn native_standard_constants_declare_their_semantics_without_enum_shapes() {
    let registry = standard_registry();
    let mut count = 0;
    for symbol in registry.symbols() {
        if let StdDecl::Value(value) = &symbol.decl
            && let Some(role) = value.runtime_role
        {
            count += 1;
            assert_eq!(value.ty, StdType::Named(role.type_path().join(".")));
            assert_eq!(value.name, role.value_name());
            let owner = registry.lookup_qualified(role.type_path()).unwrap();
            assert!(
                matches!(&owner.decl, StdDecl::Type(owner) if owner.kind == TypeDeclKind::Support && owner.representation.is_none())
            );
        }
    }
    assert_eq!(count, 2);
}

#[test]
fn native_standard_constant_descriptors_reject_wrong_owner_and_identity() {
    for defect in 0..5 {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "host", "sandbox"], "test");
        if defect != 0 {
            let mut owner = TypeDecl::generic("SandboxProfile", &[], TypeDeclKind::Support);
            if defect == 1 {
                owner.representation = Some(StdType::parse("string"));
            }
            builder.symbol(
                module,
                "SandboxProfile",
                StdSymbolKind::Type,
                StdDecl::Type(owner),
                "test",
            );
        }
        let mut value = ValueDecl::runtime(StdValueRole::DefaultCommandSandbox);
        if defect == 2 {
            value.ty = StdType::parse("string");
        }
        if defect == 3 {
            value.name = "Wrong".into();
        }
        builder.symbol(
            module,
            if defect == 4 {
                "Wrong"
            } else {
                "DefaultCommandSandbox"
            },
            StdSymbolKind::Value,
            StdDecl::Value(value),
            "test",
        );
        assert!(
            builder
                .try_finish()
                .unwrap_err()
                .reason
                .contains("standard runtime value")
        );
    }
}
