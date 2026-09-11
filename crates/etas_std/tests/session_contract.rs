use etas_std::{StdDecl, StdType};

#[test]
fn session_registry_contains_publication_not_automatic_compaction() {
    let registry = etas_std::standard_registry();
    for name in [
        "history_page",
        "prepare_context",
        "publish_context",
        "reconcile_context",
        "SummaryPlusRecent",
        "Days",
    ] {
        assert!(
            registry
                .lookup_qualified(&["std", "agent", "session", name])
                .is_some(),
            "{name}"
        );
    }
    for name in ["CompactionPolicy", "SummarizeWhen", "compact"] {
        assert!(
            registry
                .lookup_qualified(&["std", "agent", "session", name])
                .is_none(),
            "{name}"
        );
    }
    let symbol = registry
        .lookup_qualified(&["std", "agent", "session", "SessionConfig"])
        .unwrap();
    let StdDecl::Type(decl) = &symbol.decl else {
        panic!("SessionConfig declaration")
    };
    let Some(StdType::Record(fields)) = &decl.representation else {
        panic!("SessionConfig representation")
    };
    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["id", "context", "retention"]
    );
}
