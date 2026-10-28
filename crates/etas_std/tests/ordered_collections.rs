use etas_std::{FlowSourceMethodKind, StdDecl, StdType, standard_registry};

#[test]
fn ordered_constructor_overloads_use_structured_pure_key_comparison() {
    let registry = standard_registry();
    for (receiver, key, parameters) in [
        ("OrderedMap[K, V]", "K", vec!["K", "V"]),
        ("OrderedSet[T]", "T", vec!["T"]),
    ] {
        let constructors = registry
            .symbols()
            .filter_map(|symbol| match &symbol.decl {
                StdDecl::Flow(flow)
                    if flow.source_method.as_ref().is_some_and(|method| {
                        method.kind == FlowSourceMethodKind::TypeMember
                            && method.name == "new"
                            && method.receiver == StdType::parse(receiver)
                    }) =>
                {
                    Some(flow)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(constructors.len(), 2, "{receiver}");
        assert!(constructors.iter().any(|flow| flow.params.is_empty()));
        let compared = constructors
            .iter()
            .find(|flow| flow.params.len() == 1)
            .unwrap();
        assert_eq!(
            compared
                .type_params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>(),
            parameters
        );
        assert_eq!(
            compared.params[0],
            StdType::Function {
                input: vec![StdType::parse(key), StdType::parse(key)],
                output: Box::new(StdType::parse("i32")),
                effects: vec![],
            }
        );
        assert!(compared.public_effects.is_empty());
        assert!(compared.requested_actions.is_empty());
    }
}
