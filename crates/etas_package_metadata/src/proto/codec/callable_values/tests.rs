use super::*;
use crate::{CallableValueGenericBinding, GenericParamKind, TypeKind};

fn parameter(scope: CallableValueScope, position: u32) -> CallableValueSource {
    CallableValueSource::Parameter(CallableValueParameter { scope, position })
}

fn unit() -> Type {
    Type {
        name: "unit".into(),
        ..Default::default()
    }
}

fn node() -> CallableValueNode {
    CallableValueNode {
        residual_checks: Vec::new(),
        handler_requests: Some(Default::default()),
        behavior: crate::CallableValueBehavior {
            determinism: crate::CallableValueDeterminism::Deterministic,
            independent_determinism: Some(crate::CallableValueDeterminism::Deterministic),
            host_requirements: vec![],
            interpreter_features: vec![],
        },
        parent_scope: CallableValueScope::Root,
        generic_params: vec![],
        generic_bindings: vec![],
        param_names: vec![],
        params: vec![],
        output: unit(),
        public_effects: EffectRow::default(),
        requested_actions: EffectRow::default(),
        handled_requested_actions: EffectRow::default(),
        default_handler_effects: DefaultHandlerEffects::default(),
        parameter_bindings: vec![],
        action_trace: ActionTrace::Empty,
        returned_sources: None,
    }
}

fn graph() -> CallableValueGraph {
    CallableValueGraph {
        objects: Vec::new(),
        sources: vec![CallableValueSource::Value(0)],
        values: vec![node()],
    }
}

#[test]
fn frozen_contract_sources_remain_distinct_from_revision_analysis_across_binary_replay() {
    let mut graph = graph();
    graph.sources = vec![CallableValueSource::FrozenContract(0)];
    graph.values[0].returned_sources = Some(vec![
        CallableValueSource::Value(0),
        CallableValueSource::FrozenContract(0),
    ]);
    graph.validate_references(None).unwrap();
    let bytes = to_proto(&graph).encode_to_vec();
    assert_eq!(
        from_proto(ProtoCallableValueGraph::decode(bytes.as_slice()).unwrap()).unwrap(),
        graph
    );
    graph.sources = vec![CallableValueSource::FrozenContract(1)];
    assert!(graph.validate_references(None).is_err());
}

#[test]
fn callable_graph_round_trips_captures_scoped_generics_and_recursive_returns() {
    let mut graph = graph();
    let value = &mut graph.values[0];
    value.generic_params.push(GenericParam {
        name: "T".into(),
        kind: GenericParamKind::Type,
        bounds: vec![],
    });
    value.generic_bindings = vec![
        CallableValueGenericBinding {
            label: "captured-T".into(),
            scope: CallableValueScope::Root,
            position: 0,
        },
        CallableValueGenericBinding {
            label: "local-T".into(),
            scope: CallableValueScope::Value(0),
            position: 0,
        },
        CallableValueGenericBinding {
            label: "captured-E".into(),
            scope: CallableValueScope::Root,
            position: 1,
        },
    ];
    value.param_names.push("callback".into());
    value.params.push(Type {
        kind: TypeKind::Var,
        name: "local-T".into(),
        ..Default::default()
    });
    value.output = Type {
        kind: TypeKind::Var,
        name: "captured-T".into(),
        ..Default::default()
    };
    value.public_effects.tail = Some("captured-E".into());
    value.parameter_bindings = vec![
        CallableValueParameterBinding {
            label: "captured-callback".into(),
            parameter: CallableValueParameter {
                scope: CallableValueScope::Root,
                position: 0,
            },
        },
        CallableValueParameterBinding {
            label: "own-callback".into(),
            parameter: CallableValueParameter {
                scope: CallableValueScope::Value(0),
                position: 0,
            },
        },
    ];
    value.action_trace = ActionTrace::Seq(vec![
        ActionTrace::ParameterCall {
            parameter: "captured-callback".into(),
        },
        ActionTrace::Widened {
            actions: vec![],
            parameter_calls: vec!["own-callback".into()],
        },
    ]);
    value.returned_sources = Some(vec![CallableValueSource::Value(0)]);
    let root = vec![GenericParamKind::Type, GenericParamKind::Effect];
    graph.validate_references(Some((1, &root))).unwrap();
    crate::validate_callable_value_graph(&graph).unwrap();
    let proto = to_proto(&graph);
    let bytes = proto.encode_to_vec();
    assert_eq!(
        from_proto(ProtoCallableValueGraph::decode(bytes.as_slice()).unwrap()).unwrap(),
        graph
    );
    let mut missing = graph.clone();
    missing.values[0].behavior.independent_determinism = None;
    assert!(missing.validate_references(Some((1, &root))).is_err());
    assert!(from_proto(to_proto(&missing)).is_err());
    let mut inconsistent = graph.clone();
    inconsistent.values[0].behavior.independent_determinism =
        Some(crate::CallableValueDeterminism::RuntimeMediated);
    assert!(inconsistent.validate_references(Some((1, &root))).is_err());
    assert!(from_proto(to_proto(&inconsistent)).is_err());
}

#[test]
fn callable_graph_rejects_missing_node_artifacts_instead_of_fabricating_empty_facts() {
    for field in 0..10 {
        let mut proto = to_proto(&graph());
        let node = &mut proto.values[0];
        match field {
            0 => node.output = None,
            1 => node.public_effects = None,
            2 => node.requested_actions = None,
            3 => node.handled_requested_actions = None,
            4 => node.default_handler_effects = None,
            5 => node.action_trace = None,
            6 => node.parent_scope = None,
            7 => node.behavior = None,
            8 => node.behavior.as_mut().unwrap().determinism = None,
            9 => node.residual_checks = None,
            _ => unreachable!(),
        }
        let error = from_proto(proto).unwrap_err();
        assert!(error.to_string().contains("missing"), "{field}: {error}");
    }
}

#[test]
fn residual_references_round_trip_and_reject_malformed_binary() {
    use etas_code::{
        CodeFingerprint,
        residual::identity::{
            GenericArgumentIdentity, GenericParameterId, ObligationId, ResidualObligationRef,
        },
    };
    let fp = |byte| CodeFingerprint::from_bytes([byte; 32]);
    let parameter = GenericParameterId::from_canonical_fingerprint(fp(2));
    let reference = ResidualObligationRef::new(
        ObligationId::from_canonical_fingerprint(fp(1)),
        std::collections::BTreeMap::from([(parameter, GenericArgumentIdentity::Type(fp(3)))]),
    )
    .unwrap();
    let mut source = graph();
    source.values[0].residual_checks.push(reference);
    let bytes = to_proto(&source).encode_to_vec();
    assert_eq!(
        from_proto(ProtoCallableValueGraph::decode(bytes.as_slice()).unwrap()).unwrap(),
        source
    );
    assert_ne!(
        crate::callable_value_graph_fingerprint(&source).unwrap(),
        crate::callable_value_graph_fingerprint(&graph()).unwrap()
    );
    for bad in 0..5 {
        let mut wire = to_proto(&source);
        let set = wire.values[0].residual_checks.as_mut().unwrap();
        match bad {
            0 => set.version = 0,
            1 => set.references[0].obligation.clear(),
            2 => set.references[0].substitutions[0].kind = 99,
            3 => {
                let binding = set.references[0].substitutions[0].clone();
                set.references[0].substitutions.push(binding);
            }
            4 => set.references.push(set.references[0].clone()),
            _ => unreachable!(),
        }
        let bytes = wire.encode_to_vec();
        assert!(
            from_proto(ProtoCallableValueGraph::decode(bytes.as_slice()).unwrap()).is_err(),
            "case {bad}"
        );
    }
}

#[test]
fn residual_reference_budget_is_shared_across_the_artifact() {
    use etas_code::{
        CodeFingerprint,
        residual::identity::{
            GenericArgumentIdentity, GenericParameterId, ObligationId, ResidualObligationRef,
        },
    };
    let mut bindings = std::collections::BTreeMap::new();
    for number in 0..18u8 {
        let parameter = GenericParameterId::from_canonical_fingerprint(
            CodeFingerprint::from_bytes([number; 32]),
        );
        bindings.insert(
            parameter,
            GenericArgumentIdentity::Type(CodeFingerprint::from_bytes([99; 32])),
        );
    }
    let references = (0..2700u32)
        .map(|number| {
            let mut bytes = [0u8; 32];
            bytes[..4].copy_from_slice(&number.to_be_bytes());
            ResidualObligationRef::new(
                ObligationId::from_canonical_fingerprint(CodeFingerprint::from_bytes(bytes)),
                bindings.clone(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let mut value = node();
    value.residual_checks = references;
    let graph = CallableValueGraph {
        objects: Vec::new(),
        sources: vec![CallableValueSource::Value(0)],
        values: vec![value.clone(), value],
    };
    let mut package = PackageMetadata::default();
    package
        .public_metadata
        .effect_summaries
        .push(EffectSummary {
            returned_callables: Some(graph),
            ..Default::default()
        });
    assert!(validate_package_metadata_graph(&package).is_err());
    assert!(validate_proto_package_metadata_graph(&package_metadata_to_proto(&package)).is_err());
}

#[test]
fn root_and_latent_residuals_share_the_artifact_budget() {
    use etas_code::{
        CodeFingerprint,
        residual::identity::{
            GenericArgumentIdentity, GenericParameterId, ObligationId, ResidualObligationRef,
        },
    };
    let bindings: std::collections::BTreeMap<_, _> = (0..18u8)
        .map(|number| {
            (
                GenericParameterId::from_canonical_fingerprint(CodeFingerprint::from_bytes(
                    [number; 32],
                )),
                GenericArgumentIdentity::Type(CodeFingerprint::from_bytes([99; 32])),
            )
        })
        .collect();
    let references = (0..2700u32)
        .map(|number| {
            let mut bytes = [0u8; 32];
            bytes[..4].copy_from_slice(&number.to_be_bytes());
            ResidualObligationRef::new(
                ObligationId::from_canonical_fingerprint(CodeFingerprint::from_bytes(bytes)),
                bindings.clone(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let mut package = PackageMetadata::default();
    package
        .public_metadata
        .effect_summaries
        .push(EffectSummary {
            residual_checks: references.clone(),
            latent_flows: vec![crate::LatentFlowSummary::default()],
            ..Default::default()
        });
    assert!(validate_package_metadata_graph(&package).is_ok());
    assert!(validate_proto_package_metadata_graph(&package_metadata_to_proto(&package)).is_ok());
    package.public_metadata.effect_summaries[0].latent_flows[0].residual_checks = references;
    assert!(validate_package_metadata_graph(&package).is_err());
    assert!(validate_proto_package_metadata_graph(&package_metadata_to_proto(&package)).is_err());
}

#[test]
fn callable_behavior_has_typed_closed_enums_and_affects_content_identity() {
    let original = graph();
    let before = crate::callable_value_graph_fingerprint(&original).unwrap();
    let mut changed = original.clone();
    changed.values[0].behavior.determinism = crate::CallableValueDeterminism::RuntimeMediated;
    changed.values[0]
        .behavior
        .host_requirements
        .push(crate::CallableValueHostRequirement::Compiler);
    changed.values[0]
        .behavior
        .interpreter_features
        .push(crate::CallableValueInterpreterFeature::Retry);
    assert_eq!(from_proto(to_proto(&changed)).unwrap(), changed);
    assert_ne!(
        crate::callable_value_graph_fingerprint(&changed).unwrap(),
        before
    );
    for field in 0..3 {
        let mut proto = to_proto(&original);
        let behavior = proto.values[0].behavior.as_mut().unwrap();
        match field {
            0 => behavior.determinism = Some(999),
            1 => behavior.host_requirements.push(999),
            _ => behavior.interpreter_features.push(999),
        }
        assert!(
            from_proto(proto)
                .unwrap_err()
                .to_string()
                .contains("unknown")
        );
    }
    changed.values[0]
        .behavior
        .host_requirements
        .push(crate::CallableValueHostRequirement::Compiler);
    assert!(
        crate::validate_callable_value_graph(&changed)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
}

#[test]
fn callable_graph_rejects_dangling_scopes_empty_returns_and_unbound_traces() {
    let mut cases = vec![];
    let mut bad = graph();
    bad.sources = vec![CallableValueSource::Value(1)];
    cases.push(bad);
    let mut bad = graph();
    bad.sources = vec![parameter(CallableValueScope::Value(0), 0)];
    cases.push(bad);
    let mut bad = graph();
    bad.values[0].returned_sources = Some(vec![]);
    cases.push(bad);
    let mut bad = graph();
    bad.values[0].action_trace = ActionTrace::ParameterCall {
        parameter: "guessed-by-name".into(),
    };
    cases.push(bad);
    let mut bad = graph();
    bad.values[0].action_trace = ActionTrace::Widened {
        actions: vec![],
        parameter_calls: vec!["missing".into()],
    };
    cases.push(bad);
    let mut bad = graph();
    bad.values[0].output = Type {
        kind: TypeKind::Var,
        name: "undeclared".into(),
        ..Default::default()
    };
    cases.push(bad);
    let mut bad = graph();
    bad.values[0]
        .generic_bindings
        .push(CallableValueGenericBinding {
            label: "T".into(),
            scope: CallableValueScope::Value(0),
            position: 0,
        });
    cases.push(bad);
    let mut bad = graph();
    bad.values[0].output = Type {
        kind: TypeKind::Named,
        ..Default::default()
    };
    cases.push(bad);
    let mut bad = graph();
    bad.values[0].output = Type::default();
    cases.push(bad);
    for (index, bad) in cases.into_iter().enumerate() {
        assert!(
            crate::validate_callable_value_graph(&bad).is_err(),
            "model case {index}"
        );
        assert!(from_proto(to_proto(&bad)).is_err(), "wire case {index}");
    }
}

#[test]
fn root_parameter_positions_and_generic_kinds_are_rechecked_after_binary_decode() {
    let mut graph = graph();
    graph.values[0]
        .generic_bindings
        .push(CallableValueGenericBinding {
            label: "E".into(),
            scope: CallableValueScope::Root,
            position: 0,
        });
    graph.values[0].requested_actions.tail = Some("E".into());
    let root = vec![GenericParamKind::Type];
    let decoded = from_proto(to_proto(&graph)).unwrap();
    let error = decoded.validate_references(Some((1, &root))).unwrap_err();
    assert!(error.to_string().contains("wrong kind"), "{error}");
    let bad = CallableValueGraph {
        objects: Vec::new(),
        sources: vec![parameter(CallableValueScope::Root, 1)],
        values: vec![],
    };
    let mut package = ProtoPackageGraphSection {
        version: 1,
        package: Some(ProtoPackageIdentity {
            name: "demo".into(),
            version: "1".into(),
            edition: "2026".into(),
        }),
        public_metadata: Some(ProtoPublicMetadataSection {
            flows: vec![ProtoCallableSignature {
                path: vec!["select".into()],
                input: vec![type_to_proto(&unit())],
                visibility: "public".into(),
                ..Default::default()
            }],
            effect_summaries: vec![effect_summary_to_proto(&EffectSummary {
                item: vec!["select".into()],
                returned_callables: Some(bad),
                ..Default::default()
            })],
            ..Default::default()
        }),
        ..Default::default()
    };
    let bytes = package.encode_to_vec();
    let decoded = package_metadata_from_package_graph_payload(&bytes).unwrap();
    let error = decoded.public_metadata.effect_summaries[0]
        .returned_callables
        .as_ref()
        .unwrap()
        .validate_references(Some((1, &[])))
        .unwrap_err();
    assert!(
        error.to_string().contains("not declared by its scope"),
        "{error}"
    );
    package.public_metadata.as_mut().unwrap().effect_summaries[0].returned_callables =
        Some(to_proto(&CallableValueGraph {
            objects: Vec::new(),
            sources: vec![parameter(CallableValueScope::Root, 0)],
            values: vec![],
        }));
    assert!(package_metadata_from_package_graph_payload(&package.encode_to_vec()).is_ok());
}

#[test]
fn long_callable_reference_cycles_are_flat_not_recursive_wire_graphs() {
    let count = 2000;
    let graph = CallableValueGraph {
        objects: Vec::new(),
        sources: vec![CallableValueSource::Value(0)],
        values: (0..count)
            .map(|index| {
                let mut node = node();
                node.returned_sources = Some(vec![CallableValueSource::Value((index + 1) % count)]);
                node
            })
            .collect(),
    };
    crate::validate_callable_value_graph(&graph).unwrap();
    assert_eq!(from_proto(to_proto(&graph)).unwrap(), graph);
}

#[test]
fn node_types_and_traces_share_the_artifact_depth_budget() {
    for trace in [false, true] {
        let mut graph = graph();
        for _ in 0..70 {
            if trace {
                graph.values[0].action_trace =
                    ActionTrace::Repeat(Box::new(graph.values[0].action_trace.clone()));
            } else {
                graph.values[0].output = Type {
                    kind: TypeKind::Array,
                    children: vec![graph.values[0].output.clone()],
                    ..Default::default()
                };
            }
        }
        let package = PackageMetadata {
            public_metadata: PublicMetadata {
                effect_summaries: vec![EffectSummary {
                    returned_callables: Some(graph.clone()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            ..Default::default()
        };
        let error = package_metadata_to_sections(&package).unwrap_err();
        assert!(error.to_string().contains("maximum depth"), "{error}");
        let wire = ProtoPackageGraphSection {
            public_metadata: Some(ProtoPublicMetadataSection {
                effect_summaries: vec![ProtoEffectSummary {
                    returned_callables: Some(to_proto(&graph)),
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let error =
            super::super::super::budget::validate_proto_package_metadata_graph(&wire).unwrap_err();
        assert!(error.to_string().contains("maximum depth"), "{error}");
    }
}

#[test]
fn multiple_return_graphs_consume_one_whole_artifact_node_budget() {
    let sources = (0..60_000)
        .map(|position| parameter(CallableValueScope::Root, position))
        .collect();
    let graph = CallableValueGraph {
        sources,
        values: vec![],
        objects: vec![],
    };
    let package = PackageMetadata {
        public_metadata: PublicMetadata {
            effect_summaries: vec![
                EffectSummary {
                    returned_callables: Some(graph.clone()),
                    ..Default::default()
                },
                EffectSummary {
                    returned_callables: Some(graph),
                    ..Default::default()
                },
            ],
            ..Default::default()
        },
        ..Default::default()
    };
    let error = package_metadata_to_sections(&package).unwrap_err();
    assert!(error.to_string().contains("maximum node count"), "{error}");
    let wire = ProtoPackageGraphSection {
        public_metadata: Some(ProtoPublicMetadataSection {
            effect_summaries: package
                .public_metadata
                .effect_summaries
                .iter()
                .map(effect_summary_to_proto)
                .collect(),
            ..Default::default()
        }),
        ..Default::default()
    };
    let error =
        super::super::super::budget::validate_proto_package_metadata_graph(&wire).unwrap_err();
    assert!(error.to_string().contains("maximum node count"), "{error}");
}

#[test]
fn object_evidence_round_trips_bound_and_concrete_member_witnesses() {
    let package = etas_code::PackageKey::from_name_and_version("objects", "1.0.0").unwrap();
    let declaration = |name: &str, kind| {
        etas_code::DefinitionKey::new(package, vec!["api".into()], vec![name.into()], kind).unwrap()
    };
    let method_identity = etas_code::MethodId::new(
        etas_code::ImplementationKey::new(
            package,
            vec!["api".into()],
            declaration("Label", etas_code::DefinitionKind::Type),
            None,
            etas_code::CodeFingerprint::of_bytes("test.object-member.header", b"Label~Named"),
        )
        .unwrap(),
        "name".into(),
    )
    .unwrap();
    let bound = crate::SpecBound {
        spec: vec!["api".into(), "Named".into()],
        args: vec![],
    };
    let interface = Type {
        kind: TypeKind::Existential,
        bounds: vec![bound.clone()],
        ..Default::default()
    };
    let producer = Type {
        kind: TypeKind::Nominal,
        path: vec!["api".into(), "Label".into()],
        ..Default::default()
    };
    let mut member = node();
    member.param_names = vec!["self".into()];
    member.params = vec![producer.clone()];
    member.output = Type {
        name: "string".into(),
        ..Default::default()
    };
    let mut graph = CallableValueGraph {
        sources: vec![
            CallableValueSource::Existential(0),
            CallableValueSource::Existential(1),
        ],
        values: vec![member],
        objects: vec![
            ExistentialValueNode {
                parent_scope: CallableValueScope::Root,
                generic_bindings: vec![CallableValueGenericBinding {
                    label: "T".into(),
                    scope: CallableValueScope::Root,
                    position: 0,
                }],
                object_type: interface.clone(),
                source_type: Type {
                    kind: TypeKind::Var,
                    name: "T".into(),
                    ..Default::default()
                },
                payload_sources: vec![parameter(CallableValueScope::Root, 0)],
                witnesses: vec![ExistentialMemberWitness {
                    bound: bound.clone(),
                    member: "name".into(),
                    target: ExistentialWitnessTarget::Bound,
                }],
            },
            ExistentialValueNode {
                parent_scope: CallableValueScope::Root,
                generic_bindings: vec![],
                object_type: interface,
                source_type: producer,
                payload_sources: vec![],
                witnesses: vec![ExistentialMemberWitness {
                    bound,
                    member: "name".into(),
                    target: ExistentialWitnessTarget::Implementation {
                        method: method_identity.clone(),
                        value: 0,
                    },
                }],
            },
        ],
    };
    graph
        .validate_references(Some((1, &[GenericParamKind::Type])))
        .unwrap();
    crate::validate_callable_value_graph(&graph).unwrap();
    let bytes = to_proto(&graph).encode_to_vec();
    let decoded = from_proto(ProtoCallableValueGraph::decode(bytes.as_slice()).unwrap()).unwrap();
    assert_eq!(decoded, graph);
    let fingerprint = crate::callable_value_graph_input_fingerprint(&graph);
    let metadata = PackageMetadata {
        package: crate::PackageIdentity {
            name: "objects".into(),
            version: "1.0.0".into(),
            edition: "2026".into(),
        },
        public_metadata: PublicMetadata {
            effect_summaries: vec![EffectSummary {
                item: vec!["api".into(), "make".into()],
                returned_callables: Some(graph.clone()),
                ..Default::default()
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let header = crate::MetadataArtifactHeader {
        artifact_schema_version: crate::ARTIFACT_SCHEMA_VERSION,
        compiler_version: "test-compiler".into(),
        package_id: "objects".into(),
        package_version: "1.0.0".into(),
        source_payload_hash: "source".into(),
        manifest_hash: "manifest".into(),
        dependency_lock_hash: "lock".into(),
        created_target: "test".into(),
    };
    let artifact = crate::encode_metadata_artifact(
        &header,
        crate::package_metadata_to_sections(&metadata).unwrap(),
    )
    .unwrap();
    let (_, decoded) =
        crate::package_metadata_from_artifact(std::path::Path::new("objects.etasmeta"), &artifact)
            .unwrap();
    assert_eq!(decoded, metadata);
    let mut oversized = metadata.clone();
    let source = &mut oversized.public_metadata.effect_summaries[0]
        .returned_callables
        .as_mut()
        .unwrap()
        .objects[0]
        .source_type;
    for _ in 0..crate::MAX_METADATA_GRAPH_DEPTH + 1 {
        *source = Type {
            kind: TypeKind::Array,
            children: vec![std::mem::take(source)],
            ..Default::default()
        };
    }
    assert!(crate::package_metadata_to_sections(&oversized).is_err());
    graph.objects[0].witnesses[0].target = ExistentialWitnessTarget::Implementation {
        method: method_identity.clone(),
        value: 0,
    };
    assert_ne!(
        fingerprint,
        crate::callable_value_graph_input_fingerprint(&graph)
    );
    graph.remap_paths(|_, path| {
        path.iter()
            .map(|part| {
                if part == "api" {
                    "consumer".into()
                } else {
                    part.clone()
                }
            })
            .collect()
    });
    assert_eq!(
        graph.objects[0].witnesses[0].bound.spec,
        ["consumer", "Named"]
    );
    assert_eq!(
        graph.objects[1].object_type.bounds[0].spec,
        ["consumer", "Named"]
    );
    graph.objects[1].witnesses[0].target = ExistentialWitnessTarget::Implementation {
        method: method_identity,
        value: 99,
    };
    assert!(from_proto(to_proto(&graph)).is_err());
}

#[test]
fn lexical_scope_edges_cannot_capture_siblings_or_future_arguments() {
    let mut graph = graph();
    graph.values[0].params.push(unit());
    graph.values[0].param_names.push("callback".into());
    let mut nested = node();
    nested.parent_scope = CallableValueScope::Value(0);
    nested
        .parameter_bindings
        .push(CallableValueParameterBinding {
            label: "captured".into(),
            parameter: CallableValueParameter {
                scope: CallableValueScope::Value(0),
                position: 0,
            },
        });
    graph.values.push(nested);
    graph.values[0].returned_sources = Some(vec![CallableValueSource::Value(1)]);
    crate::validate_callable_value_graph(&graph).unwrap();
    graph.values[1].parent_scope = CallableValueScope::Root;
    let error = crate::validate_callable_value_graph(&graph).unwrap_err();
    assert!(error.to_string().contains("lexical scope"), "{error}");
    graph.values[1].parent_scope = CallableValueScope::Value(0);
    graph.sources = vec![parameter(CallableValueScope::Value(0), 0)];
    let error = crate::validate_callable_value_graph(&graph).unwrap_err();
    assert!(error.to_string().contains("lexical scope"), "{error}");
}

#[test]
fn lexical_scope_cycles_are_distinct_from_valid_return_reference_cycles() {
    let mut graph = graph();
    graph.values[0].parent_scope = CallableValueScope::Value(0);
    let error = crate::validate_callable_value_graph(&graph).unwrap_err();
    // A node with no root lexical owner is unavailable even before expansion.
    assert!(error.to_string().contains("lexical scope"), "{error}");
}

#[test]
fn graph_path_relocation_preserves_binders_and_reference_identity() {
    let mut graph = graph();
    graph.values[0].output = Type {
        kind: TypeKind::Named,
        path: vec!["library".into(), "Row".into()],
        ..Default::default()
    };
    graph.values[0].requested_actions.effects.push(EffectRef {
        path: vec!["library".into(), "Probe".into(), "read".into()],
        ..Default::default()
    });
    graph.remap_paths(|_, path| {
        std::iter::once("first".into())
            .chain(path.iter().cloned())
            .collect()
    });
    assert_eq!(graph.values[0].output.path, ["first", "library", "Row"]);
    assert_eq!(
        graph.values[0].requested_actions.effects[0].path,
        ["first", "library", "Probe", "read"]
    );
    assert_eq!(graph.sources, [CallableValueSource::Value(0)]);
    assert_eq!(from_proto(to_proto(&graph)).unwrap(), graph);
}
