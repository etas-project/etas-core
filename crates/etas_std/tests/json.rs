use etas_std::{StdDecl, StdType, TypeDeclKind, standard_registry};

#[test]
fn json_error_constructor_has_one_canonical_enum_identity() {
    let registry = standard_registry();
    let owner = registry
        .lookup_qualified(&["std", "json", "JsonError"])
        .unwrap();
    assert!(
        matches!(&owner.decl, StdDecl::Type(declaration) if declaration.kind == TypeDeclKind::Enum)
    );
    let constructor = registry.enum_constructor(owner.id, "InvalidJson").unwrap();
    assert_eq!(constructor.enum_owner, Some(owner.id));
    assert_eq!(
        registry
            .lookup_qualified(&["std", "json", "InvalidJson"])
            .unwrap()
            .id,
        constructor.id
    );
    assert_eq!(
        constructor.qualified_path,
        ["std", "json", "JsonError", "InvalidJson"]
    );
    assert!(
        constructor.intrinsic.is_none(),
        "enum constructors use the checked enum ABI"
    );
    let StdDecl::Flow(signature) = &constructor.decl else {
        panic!("constructor signature")
    };
    assert_eq!(signature.params, [StdType::parse("string")]);
    assert_eq!(
        signature.output,
        StdType::Named("std.json.JsonError".into())
    );
}
