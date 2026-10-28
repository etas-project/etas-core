use etas_std::{
    RuntimeTypeRole as Role, StdDecl, StdPrimitiveType, StdType, TypeDeclKind, standard_registry,
};

#[test]
fn session_and_limit_values_have_explicit_checked_representations() {
    let registry = standard_registry();
    let session_id = registry
        .lookup_qualified(&["std", "agent", "session", "SessionId"])
        .unwrap();
    let StdDecl::Type(decl) = &session_id.decl else {
        panic!("type");
    };
    assert_eq!(decl.runtime_role, Some(Role::SessionId));
    assert_eq!(
        decl.representation,
        Some(StdType::Primitive(StdPrimitiveType::String))
    );
    for (path, role, variants) in [
        (
            &["std", "agent", "session", "ContextPolicy"][..],
            Role::ContextPolicy,
            &["All", "LastTurns", "SummaryPlusRecent"][..],
        ),
        (
            &["std", "agent", "session", "RetentionPolicy"][..],
            Role::RetentionPolicy,
            &["Forever", "Days"][..],
        ),
        (
            &["std", "runtime", "limits", "Limit"][..],
            Role::Limit,
            &[
                "Iterations",
                "Tokens",
                "ContextTokens",
                "Cost",
                "WallTime",
                "Attempts",
            ][..],
        ),
    ] {
        let owner = registry.lookup_qualified(path).unwrap();
        let StdDecl::Type(decl) = &owner.decl else {
            panic!("type");
        };
        assert_eq!(decl.kind, TypeDeclKind::Enum);
        assert_eq!(decl.runtime_role, Some(role));
        let actual = registry
            .enum_constructors(owner.id)
            .map(|constructor| constructor.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(actual, variants);
        for constructor in registry.enum_constructors(owner.id) {
            let StdDecl::Flow(flow) = &constructor.decl else {
                panic!("constructor");
            };
            assert_eq!(flow.output, StdType::Named(path.join(".")));
            assert_eq!(
                flow.params.len(),
                usize::from(!matches!(flow.name.as_str(), "All" | "Forever"))
            );
        }
    }
}
