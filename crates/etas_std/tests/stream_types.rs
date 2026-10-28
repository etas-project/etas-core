use etas_std::{StdDecl, TypeDeclKind, standard_registry};

#[test]
fn stream_result_and_error_have_canonical_enum_payloads() {
    let registry = standard_registry();
    for (name, expected) in [
        ("StreamRead", vec![("Data", vec!["bytes"]), ("Eof", vec![])]),
        (
            "StreamError",
            vec![
                ("TimedOut", vec![]),
                ("Cancelled", vec![]),
                ("Closed", vec![]),
                ("Interrupted", vec![]),
                ("LimitExceeded", vec![]),
                ("Host", vec!["string"]),
            ],
        ),
    ] {
        let owner = registry.lookup_qualified(&["std", "stream", name]).unwrap();
        assert!(matches!(&owner.decl, StdDecl::Type(decl) if decl.kind == TypeDeclKind::Enum));
        let variants = registry.enum_constructors(owner.id).collect::<Vec<_>>();
        assert_eq!(variants.len(), expected.len());
        for (constructor, (name, fields)) in variants.iter().zip(expected) {
            let StdDecl::Flow(flow) = &constructor.decl else {
                panic!("enum constructor");
            };
            assert_eq!(constructor.name, name);
            assert_eq!(
                flow.params,
                fields
                    .iter()
                    .map(|ty| etas_std::StdType::parse(ty))
                    .collect::<Vec<_>>()
            );
            assert!(constructor.intrinsic.is_none());
            if owner.name == "StreamError" {
                assert_eq!(
                    registry
                        .lookup_qualified(&["std", "stream", name])
                        .unwrap()
                        .id,
                    constructor.id
                );
            }
        }
    }
}
