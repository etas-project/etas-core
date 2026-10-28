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
        callable,
        signature: TypeInfoRef(0),
        position: 0,
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
