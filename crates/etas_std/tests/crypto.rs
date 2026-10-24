use etas_std::{StdDecl, StdType, TypeDeclKind, standard_registry};

#[test]
fn digest_declares_its_nominal_bytes_representation() {
    let registry = standard_registry();
    let symbol = registry
        .lookup_qualified(&["std", "crypto", "Digest"])
        .unwrap();
    let StdDecl::Type(declaration) = &symbol.decl else {
        panic!("Digest type")
    };
    assert_eq!(declaration.kind, TypeDeclKind::Support);
    assert_eq!(declaration.representation, Some(StdType::parse("bytes")));
}
