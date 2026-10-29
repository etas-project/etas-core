use super::*;

fn invocation() -> CallableRequestTarget {
    CallableRequestTarget::SpecMethod(Box::new(SpecMethodInvocation {
        receiver: Type {
            kind: TypeKind::Var,
            name: "T".into(),
            ..Default::default()
        },
        spec: vec!["api".into(), "Encoder".into()],
        spec_args: Vec::new(),
        method: "encode".into(),
        arity: 2,
        generic_arguments: vec![
            SpecMethodGenericArgument::Type(Box::new(Type {
                kind: TypeKind::Primitive,
                name: "i32".into(),
                ..Default::default()
            })),
            SpecMethodGenericArgument::EffectRow(EffectRow {
                effects: Vec::new(),
                tail: Some("E".into()),
            }),
        ],
        callable_arguments: vec![SpecMethodCallableArgument {
            position: 1,
            sources: CallableValueGraph {
                sources: vec![CallableValueSource::Parameter(CallableValueParameter {
                    scope: CallableValueScope::Root,
                    position: 0,
                })],
                values: Vec::new(),
            },
        }],
    }))
}

#[test]
fn binary_artifact_round_trips_typed_method_trace_and_request_evidence() {
    let mut metadata = sample_metadata();
    let target = invocation();
    metadata.public_metadata.effect_summaries[0].action_trace = ActionTrace::Widened {
        actions: Vec::new(),
        parameter_calls: vec!["callback".into(), target.clone()],
    };
    metadata.public_metadata.effect_summaries[0].handler_requests =
        Some(RequestInstantiationTemplate {
            actions: Vec::new(),
            parameters: vec![ParameterRequestTemplate {
                parameter: target,
                excluded: Vec::new(),
            }],
        });
    let bytes = encode_sample(&metadata);
    let (_, decoded) =
        package_metadata_from_artifact(Path::new("package.etasmeta"), &bytes).unwrap();
    assert_eq!(decoded, metadata);
}

#[test]
fn method_evidence_path_remapping_reaches_typed_arguments_and_callback_nodes() {
    let mut target = invocation();
    let CallableRequestTarget::SpecMethod(method) = &mut target else {
        unreachable!()
    };
    method.receiver = Type {
        kind: TypeKind::Named,
        path: vec!["api".into(), "Payload".into()],
        ..Default::default()
    };
    method
        .generic_arguments
        .push(SpecMethodGenericArgument::Type(Box::new(Type {
            kind: TypeKind::Applied,
            path: vec!["api".into(), "Box".into()],
            children: vec![Type {
                kind: TypeKind::Named,
                path: vec!["api".into(), "Item".into()],
                ..Default::default()
            }],
            ..Default::default()
        })));
    let mut visited = Vec::new();
    target.remap_paths(|kind, path| {
        visited.push((kind, path.to_vec()));
        std::iter::once("dep".into())
            .chain(path.iter().cloned())
            .collect()
    });
    let CallableRequestTarget::SpecMethod(method) = &target else {
        unreachable!()
    };
    assert_eq!(method.spec, ["dep", "api", "Encoder"]);
    assert_eq!(method.receiver.path, ["dep", "api", "Payload"]);
    assert!(visited.contains(&(
        CallableValuePathKind::Type,
        vec!["api".into(), "Box".into()]
    )));
    assert!(visited.contains(&(
        CallableValuePathKind::Type,
        vec!["api".into(), "Item".into()]
    )));
    assert!(matches!(
        method.callable_arguments[0].sources.sources[0],
        CallableValueSource::Parameter(CallableValueParameter {
            scope: CallableValueScope::Root,
            position: 0
        })
    ));
}

#[test]
fn encoder_rejects_malformed_method_identity_and_argument_positions() {
    for change in [0, 1, 2] {
        let CallableRequestTarget::SpecMethod(mut method) = invocation() else {
            unreachable!()
        };
        match change {
            0 => method.spec.clear(),
            1 => method.callable_arguments[0].position = method.arity,
            _ => method
                .callable_arguments
                .push(method.callable_arguments[0].clone()),
        }
        let mut metadata = sample_metadata();
        metadata.public_metadata.effect_summaries[0].action_trace = ActionTrace::ParameterCall {
            parameter: CallableRequestTarget::SpecMethod(method),
        };
        assert!(package_metadata_to_sections(&metadata).is_err());
    }
}

#[test]
fn typed_method_evidence_shares_the_artifact_node_budget() {
    let mut metadata = sample_metadata();
    let CallableRequestTarget::SpecMethod(mut method) = invocation() else {
        unreachable!()
    };
    method.spec_args = vec![
        Type {
            kind: TypeKind::Primitive,
            name: "unit".into(),
            ..Default::default()
        };
        MAX_METADATA_GRAPH_NODES
    ];
    metadata.public_metadata.effect_summaries[0].action_trace = ActionTrace::ParameterCall {
        parameter: CallableRequestTarget::SpecMethod(method),
    };
    let error = package_metadata_to_sections(&metadata).unwrap_err();
    assert!(error.to_string().contains("node count"), "{error}");
}

#[test]
fn nested_method_callback_graphs_do_not_reset_the_depth_budget() {
    let mut target = invocation();
    for _ in 0..70 {
        let CallableRequestTarget::SpecMethod(mut method) = invocation() else {
            unreachable!()
        };
        method.callable_arguments[0].sources = CallableValueGraph {
            sources: vec![CallableValueSource::Value(0)],
            values: vec![CallableValueNode {
                handler_requests: None,
                behavior: CallableValueBehavior {
                    determinism: CallableValueDeterminism::Deterministic,
                    host_requirements: Vec::new(),
                    interpreter_features: Vec::new(),
                },
                parent_scope: CallableValueScope::Root,
                generic_params: Vec::new(),
                generic_bindings: Vec::new(),
                param_names: Vec::new(),
                params: Vec::new(),
                output: Type {
                    kind: TypeKind::Primitive,
                    name: "unit".into(),
                    ..Default::default()
                },
                public_effects: Default::default(),
                requested_actions: Default::default(),
                handled_requested_actions: Default::default(),
                default_handler_effects: Default::default(),
                parameter_bindings: Vec::new(),
                returned_sources: None,
                action_trace: ActionTrace::ParameterCall { parameter: target },
            }],
        };
        target = CallableRequestTarget::SpecMethod(method);
    }
    let mut metadata = sample_metadata();
    metadata.public_metadata.effect_summaries[0].action_trace =
        ActionTrace::ParameterCall { parameter: target };
    let error = package_metadata_to_sections(&metadata).unwrap_err();
    assert!(error.to_string().contains("maximum depth"), "{error}");
}
