use super::*;
use std::sync::Arc;

pub(super) fn with_headers() -> DefinitionDescriptorSet {
    let mut set = fixture();
    let identity = key("Worker", DefinitionKind::Agent);
    let selected = key("Readable", DefinitionKind::Spec);
    let implementation = ImplementationKey::new(
        identity.package(),
        vec!["library".into()],
        identity.clone(),
        Some(selected.clone()),
        CodeFingerprint::from_bytes([8; 32]),
    )
    .unwrap();
    let run = MethodId::new(implementation.clone(), "run".into()).unwrap();
    let helper = MethodId::new(implementation, "help".into()).unwrap();
    let component = TypeInfoRef(set.graph.types.len() as u32);
    set.graph.types.push(TypeInfo::Nominal {
        definition: identity.clone(),
        parameters: vec![],
        representation: None,
    });
    let run_signature = TypeInfoRef(set.graph.types.len() as u32);
    set.graph.types.push(TypeInfo::Function(SignatureInfo {
        generics: vec![GenericParameterRef(3)],
        parameters: vec![],
        output: TypeInfoRef(0),
        declared_output: Some(TypeInfoRef(0)),
        effects: Some(EffectRowInfoRef(1)),
    }));
    let helper_signature = TypeInfoRef(set.graph.types.len() as u32);
    set.graph.types.push(TypeInfo::Function(SignatureInfo {
        generics: vec![],
        parameters: vec![ParameterInfo {
            name: Some("input".into()),
            ty: TypeInfoRef(2),
        }],
        output: TypeInfoRef(2),
        declared_output: None,
        effects: None,
    }));
    set.declarations.push(DefinitionTypeBinding {
        definition: identity.clone(),
        ty: component,
    });
    let value = Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::String(
        "shared".into(),
    )));
    let annotation = AnnotationInfo {
        path: vec!["custom".into()],
        arguments: vec![
            AnnotationArgumentInfo {
                name: None,
                value: value.clone(),
            },
            AnnotationArgumentInfo {
                name: Some("details".into()),
                value: Arc::new(StaticValueInfo::Record(vec![
                    AnnotationFieldInfo {
                        name: "label".into(),
                        value,
                    },
                    AnnotationFieldInfo {
                        name: "module".into(),
                        value: Arc::new(StaticValueInfo::Reference(
                            AnnotationReferenceInfo::Module {
                                package: identity.package(),
                                path: vec!["library".into()],
                            },
                        )),
                    },
                    AnnotationFieldInfo {
                        name: "symbol".into(),
                        value: Arc::new(StaticValueInfo::Reference(
                            AnnotationReferenceInfo::Declaration(key(
                                "visit",
                                DefinitionKind::Flow,
                            )),
                        )),
                    },
                ])),
            },
            AnnotationArgumentInfo {
                name: Some("limits".into()),
                value: Arc::new(StaticValueInfo::Constructor {
                    kind: AnnotationConstructorKind::Limit,
                    target: AnnotationReferenceInfo::Declaration(key(
                        "limit",
                        DefinitionKind::Flow,
                    )),
                    arguments: vec![AnnotationArgumentInfo {
                        name: None,
                        value: Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::Integer(
                            "42".into(),
                        ))),
                    }],
                }),
            },
            AnnotationArgumentInfo {
                name: Some("values".into()),
                value: Arc::new(StaticValueInfo::Sequence {
                    kind: AnnotationSequenceKind::Array,
                    elements: vec![
                        Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::Unit)),
                        Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::Bool(true))),
                        Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::Float(
                            "1.5".into(),
                        ))),
                        Arc::new(StaticValueInfo::Literal(AnnotationLiteralInfo::Character(
                            'x',
                        ))),
                    ],
                }),
            },
        ],
    };
    let analysis = CallableAnalysisInfo {
        effects: EffectSummaryInfo {
            declared_bound: Some(EffectRowInfoRef(0)),
            escaping: Some(EffectRowInfoRef(0)),
            requested: Some(EffectRowInfoRef(1)),
            default_handled: Some(EffectRowInfoRef(1)),
        },
        determinism: Some(DeterminismInfo::RuntimeMediated),
        independent_determinism: Some(DeterminismInfo::Deterministic),
        trace: Some(TraceInfoRef(8)),
        outcomes: Some(TraceInfoRef(2)),
    };
    set.headers = Some(DefinitionHeaders {
        bodies: None,
        snapshots: None,
        callables: vec![
            CallableHeaderInfo {
                id: CallableId::declaration(key("visit", DefinitionKind::Flow)).unwrap(),
                signature: TypeInfoRef(10),
                analysis: analysis.clone(),
            },
            CallableHeaderInfo {
                id: CallableId::method(run.clone()),
                signature: run_signature,
                analysis: analysis.clone(),
            },
            CallableHeaderInfo {
                id: CallableId::method(helper.clone()),
                signature: helper_signature,
                analysis,
            },
        ],
        agents: vec![AgentHeaderInfo {
            method_info: None,
            identity,
            component,
            default_entry: Some(CallableId::method(run)),
            methods: vec![CallableId::method(helper)],
            tools: vec![key("Search", DefinitionKind::Tool)],
            annotations: vec![annotation],
            specs: vec![
                AttachedSpecInfo::Named(SpecInfo {
                    definition: selected.clone(),
                    arguments: vec![TypeInfoRef(2)],
                }),
                AttachedSpecInfo::InlineTrace(Arc::new(TraceConstraintInfo {
                    alternatives: vec![vec![
                        TraceRuleInfo::Allow(TracePatternInfo {
                            row: EffectRowInfoRef(1),
                            bounds: vec![TraceSelectorBoundInfo {
                                target: set.graph.rows[1].effects[0].target.clone(),
                                argument: 0,
                                spec: SpecInfo {
                                    definition: selected,
                                    arguments: vec![TypeInfoRef(2)],
                                },
                            }],
                        }),
                        TraceRuleInfo::Limit { count: 3 },
                    ]],
                })),
                AttachedSpecInfo::Protocol {
                    definition: key("Conversation", DefinitionKind::Protocol),
                    arguments: vec![TypeInfoRef(2)],
                },
            ],
        }],
    });
    set
}

#[test]
fn agent_and_method_headers_roundtrip_with_exact_analysis_and_static_metadata() {
    let set = with_headers();
    set.validate(limits()).unwrap();
    let wire = to_proto(&set);
    assert_eq!(wire.version, 2);
    let decoded =
        from_proto(ProtoDescriptorGraph::decode(wire.encode_to_vec().as_slice()).unwrap()).unwrap();
    assert_eq!(decoded, set);
    assert_eq!(fingerprint(&decoded), fingerprint(&set));
    let headers = decoded.headers.unwrap();
    assert_eq!(
        headers.agents[0].annotations[0].fingerprint(),
        set.headers.unwrap().agents[0].annotations[0].fingerprint()
    );
    assert_eq!(
        headers.callables[0].analysis.effects.escaping,
        Some(EffectRowInfoRef(0))
    );
    assert_eq!(
        headers.callables[0].analysis.effects.requested,
        Some(EffectRowInfoRef(1))
    );
}

pub(super) fn with_method_selection() -> DefinitionDescriptorSet {
    let mut set = with_headers();
    let agent = &mut set.headers.as_mut().unwrap().agents[0];
    agent.method_info = Some(
        agent
            .default_entry
            .iter()
            .chain(&agent.methods)
            .map(|callable| etas_code::descriptor::AgentMethodInfo {
                callable: callable.clone(),
                inherent: false,
                is_public: true,
                specs: vec![SpecInfo {
                    definition: key("Readable", DefinitionKind::Spec),
                    arguments: vec![TypeInfoRef(2)],
                }],
            })
            .collect(),
    );
    set
}

#[test]
fn method_selection_roundtrips_with_canonical_callables_and_spec_arguments() {
    let set = with_method_selection();
    set.validate(limits()).unwrap();
    let wire = to_proto(&set);
    assert_eq!(wire.version, 5);
    let decoded =
        from_proto(ProtoDescriptorGraph::decode(wire.encode_to_vec().as_slice()).unwrap()).unwrap();
    assert_eq!(decoded, set);
    assert_eq!(fingerprint(&decoded), fingerprint(&set));
}

#[test]
fn method_selection_cannot_omit_duplicate_or_fabricate_method_facts() {
    for case in 0..5 {
        let mut set = with_method_selection();
        let info = set.headers.as_mut().unwrap().agents[0]
            .method_info
            .as_mut()
            .unwrap();
        match case {
            0 => {
                info.pop();
            }
            1 => info.push(info[0].clone()),
            2 => {
                info[0].callable =
                    CallableId::declaration(key("visit", DefinitionKind::Flow)).unwrap()
            }
            3 => info[0].specs[0].arguments[0] = TypeInfoRef(u32::MAX),
            4 => {
                let spec = info[0].specs[0].clone();
                info[0].specs.push(spec);
            }
            _ => unreachable!(),
        }
        assert!(set.validate(limits()).is_err(), "case {case}");
        assert!(from_proto(to_proto(&set)).is_err(), "case {case}");
    }
    let mut wire = to_proto(&with_method_selection());
    wire.version = 4;
    assert!(from_proto(wire).is_err());
    let mut wire = to_proto(&with_headers());
    wire.version = 5;
    assert!(from_proto(wire).is_err());
    let mut wire = to_proto(&with_method_selection());
    wire.headers.as_mut().unwrap().agents[0]
        .method_info
        .as_mut()
        .unwrap()
        .methods[0]
        .is_public = None;
    assert!(from_proto(wire).is_err());
}

#[test]
fn malformed_headers_cannot_drop_identity_analysis_or_schema_records() {
    for case in 0..12 {
        let mut wire = to_proto(&with_headers());
        match case {
            0 => {
                wire.headers.as_mut().unwrap().callables.remove(1);
            }
            1 => wire.headers.as_mut().unwrap().agents[0].component = 0,
            2 => {
                wire.headers.as_mut().unwrap().callables[0]
                    .analysis
                    .as_mut()
                    .unwrap()
                    .determinism = Some(999)
            }
            3 => wire.version = 1,
            4 => wire.headers.as_mut().unwrap().callables[0].analysis = None,
            5 => wire.headers = None,
            6 => wire.headers.as_mut().unwrap().agents[0].default_entry = None,
            7 => {
                wire.headers.as_mut().unwrap().callables[0]
                    .analysis
                    .as_mut()
                    .unwrap()
                    .requested = Some(u32::MAX)
            }
            8 => wire.headers.as_mut().unwrap().callables[0].signature = 0,
            9 => wire.headers.as_mut().unwrap().agents[0].tools[0].kind = 1,
            10 => {
                let duplicate = wire.headers.as_ref().unwrap().agents[0].methods[0].clone();
                wire.headers.as_mut().unwrap().agents[0]
                    .methods
                    .push(duplicate);
            }
            11 => {
                wire.headers.as_mut().unwrap().agents[0].annotations[0].values[0].kind =
                    Some(schema::static_value::Kind::Character(0xD800))
            }
            _ => unreachable!(),
        }
        assert!(from_proto(wire).is_err(), "case {case}");
    }
}

#[test]
fn annotation_cycles_and_deep_graphs_are_rejected_before_recursive_values_are_built() {
    let mut annotation = schema::Annotation {
        path: vec!["custom".into()],
        arguments: vec![schema::StaticArgument {
            name: None,
            value: 0,
        }],
        values: vec![schema::StaticValue {
            kind: Some(schema::static_value::Kind::Sequence(
                schema::StaticSequence {
                    kind: 1,
                    elements: vec![0],
                },
            )),
        }],
    };
    assert!(
        annotations::decode(annotation.clone())
            .unwrap_err()
            .to_string()
            .contains("cyclic")
    );
    annotation.values = (0..66)
        .map(|i| schema::StaticValue {
            kind: Some(schema::static_value::Kind::Sequence(
                schema::StaticSequence {
                    kind: 1,
                    elements: vec![i + 1],
                },
            )),
        })
        .collect();
    annotation.values.push(schema::StaticValue {
        kind: Some(schema::static_value::Kind::Unit(true)),
    });
    assert!(
        annotations::decode(annotation)
            .unwrap_err()
            .to_string()
            .contains("maximum depth")
    );
}

#[test]
fn bare_agent_header_preserves_an_absent_default_signature() {
    let identity = key("Bare", DefinitionKind::Agent);
    let set = DefinitionDescriptorSet {
        graph: DescriptorGraphInput {
            types: vec![TypeInfo::Nominal {
                definition: identity.clone(),
                parameters: vec![],
                representation: None,
            }],
            ..Default::default()
        },
        declarations: vec![DefinitionTypeBinding {
            definition: identity.clone(),
            ty: TypeInfoRef(0),
        }],
        headers: Some(DefinitionHeaders {
            bodies: None,
            snapshots: None,
            callables: vec![],
            agents: vec![AgentHeaderInfo {
                method_info: None,
                identity,
                component: TypeInfoRef(0),
                default_entry: None,
                methods: vec![],
                tools: vec![],
                annotations: vec![],
                specs: vec![],
            }],
        }),
    };
    assert_eq!(from_proto(to_proto(&set)).unwrap(), set);
}
