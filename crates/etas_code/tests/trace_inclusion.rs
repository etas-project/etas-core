use etas_code::{
    CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, contract::*, descriptor::*,
};

fn action(name: &str, argument: Option<StaticEffectArgInfo>) -> EffectInfo {
    EffectInfo {
        target: EffectTargetInfo::Action {
            effect: DefinitionKey::new(
                PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([3; 32])),
                vec!["app".into()],
                vec!["Probe".into()],
                DefinitionKind::Effect,
            )
            .unwrap(),
            name: name.into(),
        },
        arguments: argument.into_iter().collect(),
    }
}

fn graph(traces: Vec<TraceInfo>) -> DescriptorGraph {
    DescriptorGraph::new(
        DescriptorGraphInput {
            traces,
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap()
}

fn proves(graph: &DescriptorGraph, candidate: u32, frozen: u32) -> bool {
    matches!(
        check_trace_inclusion(
            graph,
            TraceInfoRef(candidate),
            graph,
            TraceInfoRef(frozen),
            TraceProofLimits::default()
        )
        .unwrap(),
        TraceInclusion::Included
    )
}

#[test]
fn inclusion_preserves_intermediate_order_not_just_final_action_sets() {
    let graph = graph(vec![
        TraceInfo::Event(action("approve", None)),
        TraceInfo::Event(action("send", None)),
        TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(1)]),
        TraceInfo::Seq(vec![TraceInfoRef(1), TraceInfoRef(0)]),
    ]);
    assert!(proves(&graph, 2, 2));
    assert!(!proves(&graph, 3, 2));
    let TraceInclusion::NotProven(witness) = check_trace_inclusion(
        &graph,
        TraceInfoRef(3),
        &graph,
        TraceInfoRef(2),
        TraceProofLimits::default(),
    )
    .unwrap() else {
        panic!("order violation")
    };
    assert_eq!(witness.path.len(), 1);
}

#[test]
fn inclusion_accepts_branch_refinement_and_equivalent_factoring() {
    let graph = graph(vec![
        TraceInfo::Event(action("a", None)),
        TraceInfo::Event(action("b", None)),
        TraceInfo::Event(action("c", None)),
        TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(1)]),
        TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(2)]),
        TraceInfo::Choice(vec![TraceInfoRef(3), TraceInfoRef(4)]),
        TraceInfo::Choice(vec![TraceInfoRef(1), TraceInfoRef(2)]),
        TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(6)]),
        TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(0)]),
    ]);
    assert!(proves(&graph, 3, 5));
    assert!(proves(&graph, 7, 5));
    assert!(proves(&graph, 5, 7));
    assert!(
        !proves(&graph, 8, 5),
        "shared DAG nodes must not create unintended repetition"
    );
}

#[test]
fn repetition_and_epsilon_cycles_terminate_without_silent_depth_limits() {
    let graph = graph(vec![
        TraceInfo::Empty,
        TraceInfo::Repeat(TraceInfoRef(0)),
        TraceInfo::Event(action("a", None)),
        TraceInfo::Repeat(TraceInfoRef(2)),
    ]);
    assert!(proves(&graph, 1, 0));
    assert!(proves(&graph, 0, 1));
    assert!(proves(&graph, 2, 3));
    assert!(!proves(&graph, 3, 2));
    assert!(!proves(&graph, 3, 0));
}

#[test]
fn nonterminating_branches_cannot_hide_bad_prefixes() {
    let graph = graph(vec![
        TraceInfo::Empty,
        TraceInfo::Event(action("bad", None)),
        TraceInfo::Choice(vec![]),
        TraceInfo::Seq(vec![TraceInfoRef(1), TraceInfoRef(2)]),
        TraceInfo::Choice(vec![TraceInfoRef(0), TraceInfoRef(3)]),
    ]);
    assert!(!proves(&graph, 4, 0));
}

#[test]
fn symbolic_coverage_does_not_mix_partially_matching_destinations() {
    let graph = graph(vec![
        TraceInfo::Event(action("read", Some(StaticEffectArgInfo::Wildcard))),
        TraceInfo::Event(action(
            "read",
            Some(StaticEffectArgInfo::String("one".into())),
        )),
        TraceInfo::Event(action("after", None)),
        TraceInfo::Seq(vec![TraceInfoRef(1), TraceInfoRef(2)]),
        TraceInfo::Choice(vec![TraceInfoRef(0), TraceInfoRef(3)]),
        TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(2)]),
    ]);
    assert!(proves(&graph, 1, 0));
    assert!(!proves(&graph, 0, 1));
    assert!(
        !proves(&graph, 5, 4),
        "concrete suffix cannot justify every wildcard instantiation"
    );
}

#[test]
fn exhausted_proof_resources_are_errors_not_success() {
    let graph = graph(vec![TraceInfo::Event(action("a", None))]);
    for limits in [
        TraceProofLimits {
            max_automaton_states: 0,
            ..Default::default()
        },
        TraceProofLimits {
            max_transitions: 0,
            ..Default::default()
        },
        TraceProofLimits {
            max_product_states: 0,
            ..Default::default()
        },
        TraceProofLimits {
            max_product_members: 0,
            ..Default::default()
        },
        TraceProofLimits {
            max_steps: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            check_trace_inclusion(&graph, TraceInfoRef(0), &graph, TraceInfoRef(0), limits),
            Err(TraceProofError::Limit { .. })
        ));
    }
}

#[test]
fn repeated_large_labels_are_shared_in_abstract_counterexamples() {
    let payload = "s".repeat(512 * 1024);
    let mut parts = vec![TraceInfoRef(0); 128];
    parts.push(TraceInfoRef(1));
    let graph = graph(vec![
        TraceInfo::Event(action(
            "large",
            Some(StaticEffectArgInfo::String(payload.clone())),
        )),
        TraceInfo::Event(action("bad", None)),
        TraceInfo::Seq(parts),
        TraceInfo::Repeat(TraceInfoRef(0)),
    ]);
    let TraceInclusion::NotProven(witness) = check_trace_inclusion(
        &graph,
        TraceInfoRef(2),
        &graph,
        TraceInfoRef(3),
        TraceProofLimits::default(),
    )
    .unwrap() else {
        panic!("bad final action")
    };
    assert_eq!(witness.path.len(), 129);
    assert!(std::sync::Arc::ptr_eq(
        &witness.path[0].1,
        &witness.path[127].1
    ));
    assert!(
        etas_core::memory::MemoryFootprint::of(&witness).estimated_bytes()
            < (payload.len() * 2) as u64
    );
}

#[test]
fn widened_parameter_calls_are_not_erased_from_the_trace_language() {
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([3; 32])),
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
        position: 0,
        arguments: Some(vec![]),
    };
    let graph = DescriptorGraph::new(
        DescriptorGraphInput {
            types: vec![
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![ParameterInfo {
                        name: Some("f".into()),
                        ty: TypeInfoRef(1),
                    }],
                    output: TypeInfoRef(2),
                    declared_output: None,
                    effects: None,
                }),
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![],
                    output: TypeInfoRef(2),
                    declared_output: None,
                    effects: None,
                }),
                TypeInfo::Primitive(PrimitiveInfo::Unit),
            ],
            rows: vec![EffectRowInfo {
                effects: vec![],
                tail: None,
            }],
            traces: vec![
                TraceInfo::Empty,
                TraceInfo::ParameterCall(call.clone()),
                TraceInfo::Widened {
                    actions: EffectRowInfoRef(0),
                    parameter_calls: vec![call],
                },
            ],
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap();
    assert!(proves(&graph, 1, 2));
    assert!(!proves(&graph, 2, 0));
    assert!(!proves(&graph, 2, 1));
}

#[test]
fn parameter_call_arguments_are_part_of_behavior_and_unknown_is_not_empty() {
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([4; 32])),
            vec!["app".into()],
            vec!["apply".into()],
            DefinitionKind::Flow,
        )
        .unwrap(),
    )
    .unwrap();
    let call = |arguments| ParameterCallInfo {
        handlers: Some(vec![]),
        callable: callable.clone(),
        signature: TypeInfoRef(0),
        position: 0,
        arguments,
    };
    let yes = call(Some(vec![ParameterArgumentInfo::Bool(true)]));
    let no = call(Some(vec![ParameterArgumentInfo::Bool(false)]));
    let unknown = call(None);
    let input_value = call(Some(vec![ParameterArgumentInfo::Input { position: 1 }]));
    let input = DescriptorGraphInput {
        types: vec![
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![
                    ParameterInfo {
                        name: Some("f".into()),
                        ty: TypeInfoRef(1),
                    },
                    ParameterInfo {
                        name: Some("value".into()),
                        ty: TypeInfoRef(3),
                    },
                ],
                output: TypeInfoRef(2),
                declared_output: None,
                effects: Some(EffectRowInfoRef(0)),
            }),
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![ParameterInfo {
                    name: Some("value".into()),
                    ty: TypeInfoRef(3),
                }],
                output: TypeInfoRef(2),
                declared_output: None,
                effects: Some(EffectRowInfoRef(0)),
            }),
            TypeInfo::Primitive(PrimitiveInfo::Unit),
            TypeInfo::Primitive(PrimitiveInfo::Bool),
        ],
        rows: vec![EffectRowInfo {
            effects: vec![],
            tail: None,
        }],
        traces: vec![
            TraceInfo::ParameterCall(yes.clone()),
            TraceInfo::ParameterCall(no.clone()),
            TraceInfo::ParameterCall(unknown),
            TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(yes)),
            TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(no)),
            TraceInfo::ParameterCall(input_value),
        ],
        ..Default::default()
    };
    let graph = DescriptorGraph::new(input.clone(), DescriptorLimits::default()).unwrap();
    assert!(proves(&graph, 0, 0));
    assert!(!proves(&graph, 0, 1));
    assert!(!proves(&graph, 2, 2));
    assert!(!proves(&graph, 3, 4));
    assert!(proves(&graph, 5, 5));
    assert!(!proves(&graph, 5, 0));
    assert_ne!(
        graph.trace_fingerprint(TraceInfoRef(0)).unwrap(),
        graph.trace_fingerprint(TraceInfoRef(1)).unwrap()
    );
    let mut invalid = input.clone();
    invalid.types[3] = TypeInfo::Primitive(PrimitiveInfo::String);
    assert!(matches!(
        DescriptorGraph::new(invalid, DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
    let mut missing = input;
    let TraceInfo::ParameterCall(call) = &mut missing.traces[5] else {
        unreachable!()
    };
    call.arguments = Some(vec![ParameterArgumentInfo::Input { position: 42 }]);
    assert!(matches!(
        DescriptorGraph::new(missing, DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
}

#[test]
fn parameter_handler_scopes_are_part_of_the_behavioral_label() {
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([8; 32])),
            vec!["app".into()],
            vec!["apply".into()],
            DefinitionKind::Flow,
        )
        .unwrap(),
    )
    .unwrap();
    let call = |handlers| ParameterCallInfo {
        callable: callable.clone(),
        signature: TypeInfoRef(0),
        position: 0,
        arguments: Some(vec![]),
        handlers,
    };
    let unit = |row| ParameterHandlerScopeInfo::ResumeUnit {
        actions: EffectRowInfoRef(row),
    };
    let first = call(Some(vec![unit(0), unit(1)]));
    let graph = DescriptorGraph::new(
        DescriptorGraphInput {
            types: vec![
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![ParameterInfo {
                        name: Some("f".into()),
                        ty: TypeInfoRef(1),
                    }],
                    output: TypeInfoRef(2),
                    declared_output: None,
                    effects: None,
                }),
                TypeInfo::Function(SignatureInfo {
                    generics: vec![],
                    parameters: vec![],
                    output: TypeInfoRef(2),
                    declared_output: None,
                    effects: Some(EffectRowInfoRef(2)),
                }),
                TypeInfo::Primitive(PrimitiveInfo::Unit),
            ],
            rows: vec![
                EffectRowInfo {
                    effects: vec![action("a", None)],
                    tail: None,
                },
                EffectRowInfo {
                    effects: vec![action("b", None)],
                    tail: None,
                },
                EffectRowInfo {
                    effects: vec![],
                    tail: None,
                },
            ],
            traces: vec![
                TraceInfo::ParameterCall(first.clone()),
                TraceInfo::ParameterCall(call(Some(vec![unit(1), unit(0)]))),
                TraceInfo::ParameterCall(call(Some(vec![]))),
                TraceInfo::ParameterCall(call(None)),
                TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(first)),
                TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call(None))),
            ],
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap();
    assert!(proves(&graph, 0, 0));
    assert!(!proves(&graph, 0, 1));
    assert!(!proves(&graph, 0, 2));
    assert!(!proves(&graph, 3, 3));
    assert!(proves(&graph, 4, 4));
    assert!(!proves(&graph, 5, 5));
    assert_ne!(
        graph.trace_fingerprint(TraceInfoRef(0)).unwrap(),
        graph.trace_fingerprint(TraceInfoRef(1)).unwrap()
    );
    let mut missing = graph.data().clone();
    let TraceInfo::ParameterCall(call) = &mut missing.traces[0] else {
        unreachable!()
    };
    call.handlers = Some(vec![unit(42)]);
    assert!(matches!(
        DescriptorGraph::new(missing, DescriptorLimits::default()),
        Err(DescriptorError::MissingReference { .. })
    ));
    let mut not_action = graph.data().clone();
    let EffectTargetInfo::Action { effect, .. } = &not_action.rows[0].effects[0].target else {
        unreachable!()
    };
    not_action.rows[0].effects[0].target = EffectTargetInfo::Effect(effect.clone());
    assert!(matches!(
        DescriptorGraph::new(not_action, DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
}

#[test]
fn parameter_integer_values_are_type_checked_and_compared_exactly() {
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([4; 32])),
            vec!["app".into()],
            vec!["apply".into()],
            DefinitionKind::Flow,
        )
        .unwrap(),
    )
    .unwrap();
    let call = |negative, magnitude| ParameterCallInfo {
        handlers: Some(vec![]),
        callable: callable.clone(),
        signature: TypeInfoRef(0),
        position: 0,
        arguments: Some(vec![ParameterArgumentInfo::Integer {
            negative,
            magnitude,
        }]),
    };
    let input = DescriptorGraphInput {
        types: vec![
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![ParameterInfo {
                    name: None,
                    ty: TypeInfoRef(1),
                }],
                output: TypeInfoRef(2),
                declared_output: None,
                effects: Some(EffectRowInfoRef(0)),
            }),
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![ParameterInfo {
                    name: None,
                    ty: TypeInfoRef(3),
                }],
                output: TypeInfoRef(2),
                declared_output: None,
                effects: Some(EffectRowInfoRef(0)),
            }),
            TypeInfo::Primitive(PrimitiveInfo::Unit),
            TypeInfo::Primitive(PrimitiveInfo::I8),
        ],
        rows: vec![EffectRowInfo {
            effects: vec![],
            tail: None,
        }],
        traces: vec![
            TraceInfo::ParameterCall(call(true, 128)),
            TraceInfo::ParameterCall(call(true, 127)),
        ],
        ..Default::default()
    };
    let graph = DescriptorGraph::new(input.clone(), DescriptorLimits::default()).unwrap();
    assert!(proves(&graph, 0, 0));
    assert!(!proves(&graph, 0, 1));
    assert_ne!(
        graph.trace_fingerprint(TraceInfoRef(0)).unwrap(),
        graph.trace_fingerprint(TraceInfoRef(1)).unwrap()
    );
    for (negative, magnitude) in [(true, 129), (true, 0), (false, 128)] {
        let mut invalid = input.clone();
        invalid.traces[0] = TraceInfo::ParameterCall(call(negative, magnitude));
        assert!(matches!(
            DescriptorGraph::new(invalid, DescriptorLimits::default()),
            Err(DescriptorError::Invalid { .. })
        ));
    }
}

#[test]
fn parameter_nonreturn_is_distinct_from_normal_completion_and_other_callbacks() {
    let callable = etas_code::CallableId::declaration(
        DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([3; 32])),
            vec!["app".into()],
            vec!["twice".into()],
            DefinitionKind::Flow,
        )
        .unwrap(),
    )
    .unwrap();
    let call = |position| ParameterCallInfo {
        handlers: Some(vec![]),
        callable: callable.clone(),
        signature: TypeInfoRef(0),
        position,
        arguments: Some(vec![]),
    };
    let input = DescriptorGraphInput {
        types: vec![
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![
                    ParameterInfo {
                        name: Some("f".into()),
                        ty: TypeInfoRef(1),
                    },
                    ParameterInfo {
                        name: Some("g".into()),
                        ty: TypeInfoRef(1),
                    },
                ],
                output: TypeInfoRef(2),
                declared_output: None,
                effects: Some(EffectRowInfoRef(0)),
            }),
            TypeInfo::Function(SignatureInfo {
                generics: vec![],
                parameters: vec![],
                output: TypeInfoRef(2),
                declared_output: None,
                effects: Some(EffectRowInfoRef(0)),
            }),
            TypeInfo::Primitive(PrimitiveInfo::Unit),
        ],
        rows: vec![EffectRowInfo {
            effects: vec![],
            tail: None,
        }],
        traces: vec![
            TraceInfo::ParameterCall(call(0)),
            TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call(0))),
            TraceInfo::Outcome(CallableOutcomeInfo::ParameterNonReturn(call(1))),
            TraceInfo::Outcome(CallableOutcomeInfo::NonReturn),
        ],
        ..Default::default()
    };
    let graph = DescriptorGraph::new(input.clone(), DescriptorLimits::default()).unwrap();
    assert!(proves(&graph, 1, 1));
    assert!(!proves(&graph, 1, 0));
    assert!(!proves(&graph, 1, 2));
    assert!(!proves(&graph, 1, 3));
    let mut effectful = input.clone();
    effectful.rows[0].effects.push(action("write", None));
    assert!(DescriptorGraph::new(effectful, DescriptorLimits::default()).is_ok());
    let mut broad = input.clone();
    let EffectTargetInfo::Action { effect, .. } = action("write", None).target else {
        unreachable!()
    };
    broad.rows[0].effects.push(EffectInfo {
        target: EffectTargetInfo::Effect(effect),
        arguments: vec![],
    });
    assert!(DescriptorGraph::new(broad, DescriptorLimits::default()).is_err());
    let mut invalid = input;
    let TypeInfo::Function(callback) = &mut invalid.types[1] else {
        panic!("callback")
    };
    callback.parameters.push(ParameterInfo {
        name: None,
        ty: TypeInfoRef(2),
    });
    assert!(DescriptorGraph::new(invalid, DescriptorLimits::default()).is_err());
}

#[test]
fn action_order_and_return_outcome_are_compared_jointly() {
    let graph = DescriptorGraph::new(
        DescriptorGraphInput {
            types: vec![TypeInfo::Primitive(PrimitiveInfo::Bool)],
            traces: vec![
                TraceInfo::Event(action("approve", None)),
                TraceInfo::Event(action("reject", None)),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(0),
                    observation: ReturnObservationInfo::Bool(true),
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(0),
                    observation: ReturnObservationInfo::Bool(false),
                }),
                TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(2)]),
                TraceInfo::Seq(vec![TraceInfoRef(1), TraceInfoRef(3)]),
                TraceInfo::Choice(vec![TraceInfoRef(4), TraceInfoRef(5)]),
                TraceInfo::Seq(vec![TraceInfoRef(0), TraceInfoRef(3)]),
                TraceInfo::Seq(vec![TraceInfoRef(1), TraceInfoRef(2)]),
                TraceInfo::Choice(vec![TraceInfoRef(7), TraceInfoRef(8)]),
            ],
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap();
    assert!(proves(&graph, 4, 6));
    assert!(
        !proves(&graph, 9, 6),
        "same action/return sets cannot erase their correlation"
    );
}

#[test]
fn returned_raised_and_nonreturning_outcomes_are_not_conflated() {
    let graph = DescriptorGraph::new(
        DescriptorGraphInput {
            types: vec![
                TypeInfo::Primitive(PrimitiveInfo::Bool),
                TypeInfo::Primitive(PrimitiveInfo::String),
            ],
            traces: vec![
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(0),
                    observation: ReturnObservationInfo::Bool(true),
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::Returned {
                    ty: TypeInfoRef(0),
                    observation: ReturnObservationInfo::Any,
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::Raised {
                    error: TypeInfoRef(1),
                }),
                TraceInfo::Outcome(CallableOutcomeInfo::NonReturn),
            ],
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap();
    assert!(proves(&graph, 0, 1));
    assert!(!proves(&graph, 1, 0));
    assert!(!proves(&graph, 2, 1));
    assert!(!proves(&graph, 0, 2));
    assert!(!proves(&graph, 2, 3));
    assert!(!proves(&graph, 3, 2));
}
