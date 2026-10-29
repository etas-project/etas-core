use etas_code::{CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, descriptor::*};

fn action() -> EffectInfo {
    EffectInfo {
        target: EffectTargetInfo::Action {
            effect: DefinitionKey::new(
                PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
                vec!["app".into()],
                vec!["Probe".into()],
                DefinitionKind::Effect,
            )
            .unwrap(),
            name: "send".into(),
        },
        arguments: vec![],
    }
}

#[test]
fn trace_graph_rejects_missing_references_and_cycles() {
    let cyclic = DescriptorGraphInput {
        traces: vec![TraceInfo::Repeat(TraceInfoRef(0))],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(cyclic, DescriptorLimits::default()),
        Err(DescriptorError::UnguardedCycle { .. })
    ));
    let missing = DescriptorGraphInput {
        traces: vec![TraceInfo::Seq(vec![TraceInfoRef(1)])],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(missing, DescriptorLimits::default()),
        Err(DescriptorError::MissingReference { .. })
    ));
    let mut event = action();
    event
        .arguments
        .push(StaticEffectArgInfo::Type(TypeInfoRef(42)));
    let missing = DescriptorGraphInput {
        traces: vec![TraceInfo::Event(event)],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(missing, DescriptorLimits::default()),
        Err(DescriptorError::MissingReference { .. })
    ));
}

#[test]
fn trace_graph_validates_action_kind_and_counts_all_resources() {
    let mut event = action();
    let EffectTargetInfo::Action { effect, .. } = event.target else {
        unreachable!()
    };
    event.target = EffectTargetInfo::Effect(effect);
    let invalid = DescriptorGraphInput {
        traces: vec![TraceInfo::Event(event)],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(invalid, DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
    let mut deep = DescriptorGraphInput::default();
    for index in 0..10_000 {
        deep.traces.push(TraceInfo::Repeat(TraceInfoRef(index + 1)));
    }
    deep.traces.push(TraceInfo::Empty);
    assert!(matches!(
        DescriptorGraph::new(deep, DescriptorLimits::default()),
        Err(DescriptorError::Limit {
            resource: "expansion depth",
            ..
        })
    ));
    let input = DescriptorGraphInput {
        traces: vec![TraceInfo::Event(action())],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(
            input.clone(),
            DescriptorLimits {
                max_nodes: 0,
                ..Default::default()
            }
        ),
        Err(DescriptorError::Limit {
            resource: "nodes",
            ..
        })
    ));
    assert!(matches!(
        DescriptorGraph::new(
            input,
            DescriptorLimits {
                max_text_bytes: 0,
                ..Default::default()
            }
        ),
        Err(DescriptorError::Limit {
            resource: "text bytes",
            ..
        })
    ));
}

#[test]
fn trace_fingerprint_does_not_depend_on_local_trace_indices() {
    let base = DescriptorGraph::new(
        DescriptorGraphInput {
            traces: vec![
                TraceInfo::Event(action()),
                TraceInfo::Repeat(TraceInfoRef(0)),
            ],
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap();
    let reordered = DescriptorGraph::new(
        DescriptorGraphInput {
            traces: vec![
                TraceInfo::Empty,
                TraceInfo::Repeat(TraceInfoRef(2)),
                TraceInfo::Event(action()),
            ],
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap();
    assert_eq!(
        base.trace_fingerprint(TraceInfoRef(1)).unwrap(),
        reordered.trace_fingerprint(TraceInfoRef(1)).unwrap()
    );
    assert_ne!(
        base.trace_fingerprint(TraceInfoRef(1)).unwrap(),
        base.trace_fingerprint(TraceInfoRef(0)).unwrap()
    );
}

#[test]
fn root_projection_preserves_trace_events_outcomes_and_callback_placeholders() {
    let mut event = action();
    event
        .arguments
        .push(StaticEffectArgInfo::Type(TypeInfoRef(0)));
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
            vec!["app".into()],
            vec!["apply".into()],
            DefinitionKind::Flow,
        )
        .unwrap(),
    )
    .unwrap();
    let parameter = ParameterCallInfo {
        handlers: Some(vec![ParameterHandlerScopeInfo::Handle {
            arms: vec![ParameterHandlerArmInfo {
                action: event.clone(),
                trace: TraceInfoRef(0),
                completion: ParameterHandlerCompletionInfo::Resume {
                    result_type: TypeInfoRef(0),
                    value: ReturnObservationInfo::Unit,
                },
            }],
        }]),
        callable,
        signature: TypeInfoRef(1),
        position: 0,
        arguments: Some(vec![]),
    };
    let source = DescriptorGraph::new(
        DescriptorGraphInput {
            types: vec![
                TypeInfo::Primitive(PrimitiveInfo::Unit),
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![ParameterInfo {
                        name: Some("callback".into()),
                        ty: TypeInfoRef(2),
                    }],
                    output: TypeInfoRef(0),
                    declared_output: Some(TypeInfoRef(0)),
                    effects: Some(EffectRowInfoRef(0)),
                }),
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![],
                    output: TypeInfoRef(0),
                    declared_output: None,
                    effects: None,
                }),
            ],
            rows: vec![EffectRowInfo {
                effects: vec![event.clone()],
                tail: None,
            }],
            traces: vec![
                TraceInfo::Empty,
                TraceInfo::Event(event),
                TraceInfo::ParameterCall(parameter.clone()),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(0),
                    observation: ReturnObservationInfo::Unit,
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::Raised {
                    error: TypeInfoRef(0),
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::NonReturn),
                TraceInfo::UnknownOrder(EffectRowInfoRef(0)),
                TraceInfo::Widened {
                    actions: EffectRowInfoRef(0),
                    parameter_calls: vec![parameter],
                },
                TraceInfo::Seq(vec![TraceInfoRef(1), TraceInfoRef(2), TraceInfoRef(3)]),
                TraceInfo::Choice(vec![
                    TraceInfoRef(4),
                    TraceInfoRef(5),
                    TraceInfoRef(6),
                    TraceInfoRef(7),
                ]),
                TraceInfo::Repeat(TraceInfoRef(8)),
            ],
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    for index in 0..source.data().traces.len() {
        let root = TraceInfoRef(index as u32);
        let projected = source.project(DescriptorNode::Trace(root)).unwrap();
        projected.validate(Default::default()).unwrap();
        assert_eq!(
            source.trace_fingerprint(root).unwrap(),
            projected.trace_fingerprint(TraceInfoRef(0)).unwrap()
        );
    }
    assert!(
        source
            .project(DescriptorNode::Trace(TraceInfoRef(20)))
            .is_err()
    );
}

#[test]
fn parameter_trace_calls_require_a_real_signature_parameter() {
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
            vec!["app".into()],
            vec!["twice".into()],
            DefinitionKind::Flow,
        )
        .unwrap(),
    )
    .unwrap();
    let call = ParameterCallInfo {
        handlers: Some(vec![]),
        callable,
        signature: TypeInfoRef(0),
        position: 1,
        arguments: None,
    };
    let mut input = DescriptorGraphInput {
        types: vec![
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![],
                output: TypeInfoRef(1),
                declared_output: None,
                effects: None,
            }),
            TypeInfo::Primitive(PrimitiveInfo::Unit),
        ],
        traces: vec![TraceInfo::ParameterCall(call.clone())],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(input.clone(), DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
    input.traces[0] = TraceInfo::ParameterCall(ParameterCallInfo {
        signature: TypeInfoRef(42),
        ..call
    });
    assert!(matches!(
        DescriptorGraph::new(input, DescriptorLimits::default()),
        Err(DescriptorError::MissingReference { .. })
    ));
}

#[test]
fn parameter_trace_calls_require_callable_types_including_widened_traces() {
    let package = PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([9; 32]));
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            package,
            vec!["app".into()],
            vec!["apply".into()],
            DefinitionKind::Flow,
        )
        .unwrap(),
    )
    .unwrap();
    let call = ParameterCallInfo {
        handlers: Some(vec![]),
        callable,
        signature: TypeInfoRef(0),
        position: 0,
        arguments: Some(vec![]),
    };
    let mut input = DescriptorGraphInput {
        types: vec![
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![ParameterInfo {
                    name: Some("f".into()),
                    ty: TypeInfoRef(1),
                }],
                output: TypeInfoRef(1),
                declared_output: None,
                effects: None,
            }),
            TypeInfo::Primitive(PrimitiveInfo::Bool),
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![],
                output: TypeInfoRef(1),
                declared_output: None,
                effects: None,
            }),
            TypeInfo::Alias {
                definition: DefinitionKey::new(
                    package,
                    vec!["app".into()],
                    vec!["Callback".into()],
                    DefinitionKind::Type,
                )
                .unwrap(),
                parameters: vec![],
                target: TypeInfoRef(2),
            },
        ],
        rows: vec![EffectRowInfo {
            effects: vec![],
            tail: None,
        }],
        traces: vec![TraceInfo::ParameterCall(call.clone())],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(input.clone(), DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
    input.traces[0] = TraceInfo::Widened {
        actions: EffectRowInfoRef(0),
        parameter_calls: vec![call.clone()],
    };
    assert!(matches!(
        DescriptorGraph::new(input.clone(), DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
    let TypeInfo::Function(owner) = &mut input.types[0] else {
        unreachable!()
    };
    owner.parameters[0].ty = TypeInfoRef(3);
    assert!(DescriptorGraph::new(input.clone(), DescriptorLimits::default()).is_ok());
    let TypeInfo::Function(callback) = &mut input.types[2] else {
        unreachable!()
    };
    callback.effects = Some(EffectRowInfoRef(0));
    input.traces[0] = TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call));
    assert!(DescriptorGraph::new(input.clone(), DescriptorLimits::default()).is_ok());
    let TypeInfo::Alias { target, .. } = &mut input.types[3] else {
        unreachable!()
    };
    *target = TypeInfoRef(3);
    assert!(matches!(
        DescriptorGraph::new(input, DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
}

#[test]
fn outcome_observations_must_match_the_checked_type_and_constructor_owner() {
    let make = |ty, observation| DescriptorGraphInput {
        types: vec![ty],
        traces: vec![TraceInfo::Outcome(CallableOutcomeInfo::Returned {
            ty: TypeInfoRef(0),
            observation,
        })],
        ..Default::default()
    };
    assert!(
        DescriptorGraph::new(
            make(
                TypeInfo::Primitive(PrimitiveInfo::Bool),
                ReturnObservationInfo::Bool(true)
            ),
            DescriptorLimits::default()
        )
        .is_ok()
    );
    for (ty, observation) in [
        (
            TypeInfo::Primitive(PrimitiveInfo::I32),
            ReturnObservationInfo::Bool(true),
        ),
        (
            TypeInfo::Primitive(PrimitiveInfo::Never),
            ReturnObservationInfo::Any,
        ),
    ] {
        assert!(matches!(
            DescriptorGraph::new(make(ty, observation), DescriptorLimits::default()),
            Err(DescriptorError::Invalid { .. })
        ));
    }
    let owner = DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
        vec!["app".into()],
        vec!["Status".into()],
        DefinitionKind::Type,
    )
    .unwrap();
    let wrong_owner = DefinitionKey::new(
        owner.package(),
        vec!["app".into()],
        vec!["Other".into()],
        DefinitionKind::Type,
    )
    .unwrap();
    let ty = TypeInfo::Enum {
        definition: owner,
        parameters: vec![],
        variants: vec![VariantInfo {
            name: "Accepted".into(),
            payload: VariantPayloadInfo::None,
        }],
    };
    assert!(matches!(
        DescriptorGraph::new(
            make(
                ty,
                ReturnObservationInfo::Variant {
                    owner: wrong_owner,
                    name: "Accepted".into()
                }
            ),
            DescriptorLimits::default()
        ),
        Err(DescriptorError::Invalid { .. })
    ));
}
