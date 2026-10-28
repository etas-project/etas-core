use etas_std::{
    FlowDecl, FlowSourceMethodKind, StdDecl, StdEffectRef, StdGenericParam, StdRegistryBuilder,
    StdRegistryVersion, StdSymbolKind, StdType, standard_registry,
};

#[test]
fn priority_queue_constructor_overloads_have_structured_comparison_signatures() {
    let registry = standard_registry();
    let constructors = registry
        .symbols()
        .filter_map(|symbol| match &symbol.decl {
            StdDecl::Flow(flow)
                if flow.source_method.as_ref().is_some_and(|method| {
                    method.kind == FlowSourceMethodKind::TypeMember
                        && method.name == "new"
                        && method.receiver == StdType::parse("PriorityQueue[T, P]")
                }) =>
            {
                Some(flow)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(constructors.len(), 2);
    let comparison = constructors
        .iter()
        .find(|flow| flow.params.len() == 1)
        .unwrap();
    assert_eq!(
        comparison
            .type_params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>(),
        ["T", "P"]
    );
    assert_eq!(
        comparison.params[0],
        StdType::Function {
            input: vec![StdType::parse("P"), StdType::parse("P")],
            output: Box::new(StdType::parse("i32")),
            effects: vec![],
        }
    );
    assert!(comparison.public_effects.is_empty());
    assert!(comparison.requested_actions.is_empty());
}

#[test]
fn callable_descriptors_validate_parameter_result_and_effect_graphs() {
    for (input, output, effects, valid) in [
        (
            vec![StdType::parse("P")],
            StdType::parse("i32"),
            vec![],
            true,
        ),
        (
            vec![StdType::parse("Q")],
            StdType::parse("i32"),
            vec![],
            false,
        ),
        (
            vec![StdType::parse("P")],
            StdType::parse("Q"),
            vec![],
            false,
        ),
        (
            vec![StdType::parse("P")],
            StdType::parse("i32"),
            vec![StdEffectRef::new(&["Missing"])],
            false,
        ),
    ] {
        let mut builder = StdRegistryBuilder::new(StdRegistryVersion::phase1());
        let module = builder.module(&["std", "probe"], "probe");
        let mut flow = FlowDecl::with_type_params_actions(
            "apply",
            &[StdGenericParam::new("P")],
            &[],
            "unit",
            &[],
            &[],
        );
        flow.params.push(StdType::Function {
            input,
            output: Box::new(output),
            effects,
        });
        builder.symbol(
            module,
            "apply",
            StdSymbolKind::Flow,
            StdDecl::Flow(flow),
            "probe",
        );
        assert_eq!(builder.try_finish().is_ok(), valid);
    }
}
