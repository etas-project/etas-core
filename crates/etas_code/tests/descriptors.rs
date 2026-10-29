use etas_code::{CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, descriptor::*};
use etas_core::memory::MemoryFootprint;

fn key(name: &str, kind: DefinitionKind) -> DefinitionKey {
    DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
        vec!["app".into()],
        vec![name.into()],
        kind,
    )
    .unwrap()
}

fn graph(input: DescriptorGraphInput) -> DescriptorGraph {
    DescriptorGraph::new(input, DescriptorLimits::default()).unwrap()
}

fn tree() -> DescriptorGraphInput {
    let definition = key("Tree", DefinitionKind::Type);
    DescriptorGraphInput {
        types: vec![
            TypeInfo::Enum {
                definition: definition.clone(),
                parameters: vec![GenericParameterRef(0)],
                variants: vec![
                    VariantInfo {
                        name: "Leaf".into(),
                        payload: VariantPayloadInfo::Positional(vec![TypeInfoRef(1)]),
                    },
                    VariantInfo {
                        name: "Branch".into(),
                        payload: VariantPayloadInfo::Named(vec![
                            FieldInfo {
                                name: "left".into(),
                                ty: TypeInfoRef(0),
                            },
                            FieldInfo {
                                name: "right".into(),
                                ty: TypeInfoRef(0),
                            },
                        ]),
                    },
                ],
            },
            TypeInfo::Parameter(GenericParameterRef(0)),
        ],
        binders: vec![GenericParameterInfo {
            owner: GenericOwner::Declaration(definition),
            position: 0,
            name: "T".into(),
            kind: GenericParameterKind::Type,
            bounds: vec![SpecInfo {
                definition: key("Allowed", DefinitionKind::Spec),
                arguments: vec![TypeInfoRef(1)],
            }],
        }],
        rows: vec![],
        traces: vec![],
    }
}

#[test]
fn merged_descriptor_binders_preserve_identity_and_reject_conflicts() {
    let original = graph(tree());
    let roots = [DescriptorNode::Type(TypeInfoRef(0))];
    let (mut combined, first) = original
        .relocate_roots(&roots, DescriptorGraphOffsets::default())
        .unwrap();
    let offsets = DescriptorGraphOffsets {
        types: combined.types.len() as u32,
        rows: combined.rows.len() as u32,
        binders: combined.binders.len() as u32,
        traces: combined.traces.len() as u32,
    };
    let (second, second_roots) = original.relocate_roots(&roots, offsets).unwrap();
    combined.types.extend(second.types);
    combined.rows.extend(second.rows);
    combined.binders.extend(second.binders);
    combined.traces.extend(second.traces);
    assert!(DescriptorGraph::new(combined.clone(), Default::default()).is_err());
    let merged =
        DescriptorGraph::new_with_merged_binders(combined.clone(), Default::default()).unwrap();
    assert_eq!(merged.data().binders.len(), 1);
    let (DescriptorNode::Type(first), DescriptorNode::Type(second)) = (first[0], second_roots[0])
    else {
        panic!("type roots");
    };
    assert_eq!(
        merged.type_fingerprint(first).unwrap(),
        original.type_fingerprint(TypeInfoRef(0)).unwrap()
    );
    assert_eq!(
        merged.type_fingerprint(second).unwrap(),
        original.type_fingerprint(TypeInfoRef(0)).unwrap()
    );
    let mut conflict = combined.clone();
    conflict.binders[1].bounds[0].definition = key("Different", DefinitionKind::Spec);
    assert!(DescriptorGraph::new_with_merged_binders(conflict, Default::default()).is_err());
    let mut malformed = combined;
    malformed.binders[1].bounds[0].arguments[0] = TypeInfoRef(u32::MAX);
    assert!(DescriptorGraph::new_with_merged_binders(malformed, Default::default()).is_err());
}

#[test]
fn recursive_nominal_graphs_and_self_referential_spec_bounds_are_finite() {
    let graph = graph(tree());
    assert!(graph.type_info(TypeInfoRef(0)).is_some());
    assert!(graph.type_fingerprint(TypeInfoRef(0)).is_ok());
    let mutually_recursive = graph_from_types(vec![
        TypeInfo::Nominal {
            definition: key("Directory", DefinitionKind::Type),
            parameters: vec![],
            representation: Some(TypeInfoRef(1)),
        },
        TypeInfo::Record(vec![FieldInfo {
            name: "entry".into(),
            ty: TypeInfoRef(2),
        }]),
        TypeInfo::Enum {
            definition: key("Entry", DefinitionKind::Type),
            parameters: vec![],
            variants: vec![VariantInfo {
                name: "Directory".into(),
                payload: VariantPayloadInfo::Positional(vec![TypeInfoRef(0)]),
            }],
        },
    ]);
    assert!(mutually_recursive.type_fingerprint(TypeInfoRef(0)).is_ok());
}

fn graph_from_types(types: Vec<TypeInfo>) -> DescriptorGraph {
    graph(DescriptorGraphInput {
        types,
        ..Default::default()
    })
}

#[test]
fn builtin_descriptors_preserve_kind_and_validate_arity() {
    let input = DescriptorGraphInput {
        types: vec![
            TypeInfo::Primitive(PrimitiveInfo::I128),
            TypeInfo::Primitive(PrimitiveInfo::U128),
            TypeInfo::Builtin {
                constructor: BuiltinTypeInfo::Array,
                arguments: vec![TypeInfoRef(0)],
            },
            TypeInfo::Builtin {
                constructor: BuiltinTypeInfo::List,
                arguments: vec![TypeInfoRef(0)],
            },
        ],
        ..Default::default()
    };
    let graph = graph(input.clone());
    assert_ne!(
        graph.type_fingerprint(TypeInfoRef(0)).unwrap(),
        graph.type_fingerprint(TypeInfoRef(1)).unwrap()
    );
    assert_ne!(
        graph.type_fingerprint(TypeInfoRef(2)).unwrap(),
        graph.type_fingerprint(TypeInfoRef(3)).unwrap()
    );
    let mut malformed = input;
    malformed.types[2] = TypeInfo::Builtin {
        constructor: BuiltinTypeInfo::Map,
        arguments: vec![TypeInfoRef(0)],
    };
    assert!(matches!(
        DescriptorGraph::new(malformed, DescriptorLimits::default()),
        Err(DescriptorError::Invalid { .. })
    ));
}

#[test]
fn agent_components_are_nominal_but_not_generic_type_constructors() {
    let agent = key("Worker", DefinitionKind::Agent);
    let input = DescriptorGraphInput {
        types: vec![TypeInfo::Nominal {
            definition: agent.clone(),
            parameters: Vec::new(),
            representation: None,
        }],
        ..Default::default()
    };
    assert!(DescriptorGraph::new(input.clone(), DescriptorLimits::default()).is_ok());
    let mut generic = input;
    generic.types[0] = TypeInfo::Nominal {
        definition: agent.clone(),
        parameters: vec![GenericParameterRef(0)],
        representation: None,
    };
    generic.binders.push(GenericParameterInfo {
        owner: GenericOwner::Declaration(agent),
        position: 0,
        name: "T".into(),
        kind: GenericParameterKind::Type,
        bounds: Vec::new(),
    });
    assert!(DescriptorGraph::new(generic, DescriptorLimits::default()).is_err());
}

#[test]
fn descriptor_digest_does_not_encode_graph_arena_numbers() {
    let a = graph(tree());
    let mut reordered = tree();
    reordered.types.swap(0, 1);
    let TypeInfo::Enum { variants, .. } = &mut reordered.types[1] else {
        panic!("enum")
    };
    variants[0].payload = VariantPayloadInfo::Positional(vec![TypeInfoRef(0)]);
    variants[1].payload = VariantPayloadInfo::Named(vec![
        FieldInfo {
            name: "left".into(),
            ty: TypeInfoRef(1),
        },
        FieldInfo {
            name: "right".into(),
            ty: TypeInfoRef(1),
        },
    ]);
    reordered.binders[0].bounds[0].arguments = vec![TypeInfoRef(0)];
    let b = graph(reordered);
    assert_eq!(
        a.type_fingerprint(TypeInfoRef(0)).unwrap(),
        b.type_fingerprint(TypeInfoRef(1)).unwrap()
    );
}

#[test]
fn root_projection_preserves_recursive_type_and_generic_bound_identity() {
    let mut input = tree();
    input.types.push(TypeInfo::Record(vec![FieldInfo {
        name: "unrelated_private_field".into(),
        ty: TypeInfoRef(1),
    }]));
    input.rows.push(EffectRowInfo {
        effects: vec![],
        tail: None,
    });
    input.traces.push(TraceInfo::Empty);
    let source = graph(input);
    let projected = source
        .project(DescriptorNode::Type(TypeInfoRef(0)))
        .unwrap();
    projected.validate(Default::default()).unwrap();
    assert_eq!(projected.data().types.len(), 2);
    assert_eq!(projected.data().binders.len(), 1);
    assert!(projected.data().rows.is_empty());
    assert!(projected.data().traces.is_empty());
    assert_eq!(
        source.type_fingerprint(TypeInfoRef(0)).unwrap(),
        projected.type_fingerprint(TypeInfoRef(0)).unwrap()
    );
    assert_eq!(
        projected,
        projected
            .project(DescriptorNode::Type(TypeInfoRef(0)))
            .unwrap()
    );
    assert!(
        source
            .project(DescriptorNode::Type(TypeInfoRef(100)))
            .is_err()
    );
}

#[test]
fn root_projection_remaps_effect_rows_binders_and_function_signatures() {
    let source = graph(generic_flow());
    for root in [
        DescriptorNode::Type(TypeInfoRef(1)),
        DescriptorNode::Row(EffectRowInfoRef(0)),
    ] {
        let projected = source.project(root).unwrap();
        projected.validate(Default::default()).unwrap();
        match root {
            DescriptorNode::Type(ty) => assert_eq!(
                source.type_fingerprint(ty).unwrap(),
                projected.type_fingerprint(TypeInfoRef(0)).unwrap()
            ),
            DescriptorNode::Row(row) => {
                assert_eq!(
                    source.row_fingerprint(row).unwrap(),
                    projected.row_fingerprint(EffectRowInfoRef(0)).unwrap()
                );
                assert_eq!(projected.data().types.len(), 1);
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn root_projection_remaps_every_type_shape_without_changing_its_fingerprint() {
    let mut input = tree();
    input.types.extend([
        TypeInfo::Primitive(PrimitiveInfo::I32),
        TypeInfo::Applied {
            constructor: TypeInfoRef(0),
            arguments: vec![TypeInfoRef(2)],
        },
        TypeInfo::Builtin {
            constructor: BuiltinTypeInfo::Array,
            arguments: vec![TypeInfoRef(3)],
        },
        TypeInfo::Alias {
            definition: key("Forest", DefinitionKind::Type),
            parameters: vec![],
            target: TypeInfoRef(4),
        },
        TypeInfo::Tuple(vec![TypeInfoRef(3), TypeInfoRef(5)]),
        TypeInfo::Record(vec![FieldInfo {
            name: "forest".into(),
            ty: TypeInfoRef(5),
        }]),
        TypeInfo::Nominal {
            definition: key("Bag", DefinitionKind::Type),
            parameters: vec![],
            representation: Some(TypeInfoRef(7)),
        },
        TypeInfo::Handler {
            handled: EffectRowInfoRef(0),
            produced: Some(EffectRowInfoRef(1)),
            result: Some(TypeInfoRef(8)),
        },
    ]);
    input.rows.extend([
        EffectRowInfo {
            effects: vec![EffectInfo {
                target: EffectTargetInfo::Effect(key("Error", DefinitionKind::Effect)),
                arguments: vec![StaticEffectArgInfo::Type(TypeInfoRef(8))],
            }],
            tail: None,
        },
        EffectRowInfo {
            effects: vec![],
            tail: None,
        },
    ]);
    let source = graph(input);
    for index in 0..source.data().types.len() {
        let root = TypeInfoRef(index as u32);
        let projected = source.project(DescriptorNode::Type(root)).unwrap();
        projected.validate(Default::default()).unwrap();
        assert_eq!(
            source.type_fingerprint(root).unwrap(),
            projected.type_fingerprint(TypeInfoRef(0)).unwrap()
        );
    }
}

#[test]
fn diagnostic_descriptor_exports_share_only_their_actual_root_closure() {
    use etas_code::diagnostic::DescriptorDetail;
    let source = graph(DescriptorGraphInput {
        types: vec![
            TypeInfo::Primitive(PrimitiveInfo::I32),
            TypeInfo::Primitive(PrimitiveInfo::String),
            TypeInfo::Record(vec![FieldInfo {
                name: "private_catalog_field".into(),
                ty: TypeInfoRef(1),
            }]),
        ],
        rows: vec![EffectRowInfo {
            effects: vec![],
            tail: None,
        }],
        traces: vec![TraceInfo::Empty],
        ..Default::default()
    });
    for detail in [
        DescriptorDetail::Type {
            graph: source.clone(),
            ty: TypeInfoRef(1),
        },
        DescriptorDetail::EffectRow {
            graph: source.clone(),
            row: EffectRowInfoRef(0),
        },
        DescriptorDetail::Trace {
            graph: source.clone(),
            trace: TraceInfoRef(0),
        },
        DescriptorDetail::Spec {
            graph: source.clone(),
            spec: SpecInfo {
                definition: key("Allowed", DefinitionKind::Spec),
                arguments: vec![TypeInfoRef(1), TypeInfoRef(0), TypeInfoRef(1)],
            },
        },
        DescriptorDetail::Spec {
            graph: source.clone(),
            spec: SpecInfo {
                definition: key("Marker", DefinitionKind::Spec),
                arguments: vec![],
            },
        },
    ] {
        let json = serde_json::to_value(&detail).unwrap();
        assert!(!json.to_string().contains("private_catalog_field"));
        if let DescriptorDetail::Spec { spec, .. } = &detail {
            let graph = &json["Spec"]["graph"];
            let types = graph["types"].as_array().unwrap();
            if spec.arguments.is_empty() {
                assert!(types.is_empty());
            } else {
                assert_eq!(types.len(), 2);
                assert_eq!(
                    json["Spec"]["spec"]["arguments"],
                    serde_json::json!([0, 1, 0])
                );
                assert_eq!(types[0], serde_json::json!({"Primitive":"String"}));
                assert_eq!(types[1], serde_json::json!({"Primitive":"I32"}));
            }
            assert!(graph["rows"].as_array().unwrap().is_empty());
            assert!(graph["traces"].as_array().unwrap().is_empty());
        }
    }
    assert!(
        serde_json::to_value(DescriptorDetail::Type {
            graph: source,
            ty: TypeInfoRef(100)
        })
        .is_err()
    );
}

fn generic_flow() -> DescriptorGraphInput {
    let owner = GenericOwner::Declaration(key("read", DefinitionKind::Flow));
    DescriptorGraphInput {
        types: vec![
            TypeInfo::Parameter(GenericParameterRef(0)),
            TypeInfo::Function(SignatureInfo {
                generics: vec![GenericParameterRef(0), GenericParameterRef(1)],
                parameters: vec![ParameterInfo {
                    name: Some("path".into()),
                    ty: TypeInfoRef(0),
                }],
                output: TypeInfoRef(0),
                declared_output: None,
                effects: Some(EffectRowInfoRef(0)),
            }),
        ],
        rows: vec![EffectRowInfo {
            effects: vec![EffectInfo {
                target: EffectTargetInfo::Action {
                    effect: key("Fs", DefinitionKind::Effect),
                    name: "read".into(),
                },
                arguments: vec![StaticEffectArgInfo::Type(TypeInfoRef(0))],
            }],
            tail: Some(GenericParameterRef(1)),
        }],
        binders: vec![
            GenericParameterInfo {
                owner: owner.clone(),
                position: 0,
                name: "R".into(),
                kind: GenericParameterKind::Type,
                bounds: vec![],
            },
            GenericParameterInfo {
                owner,
                position: 1,
                name: "E".into(),
                kind: GenericParameterKind::EffectRow,
                bounds: vec![],
            },
        ],
        traces: vec![],
    }
}

#[test]
fn structured_rows_preserve_static_arguments_and_binder_ownership() {
    let input = generic_flow();
    let base = graph(input.clone());
    let row = base.effect_row(EffectRowInfoRef(0)).unwrap();
    assert_eq!(row.tail, Some(GenericParameterRef(1)));
    assert_eq!(
        row.effects[0].arguments,
        vec![StaticEffectArgInfo::Type(TypeInfoRef(0))]
    );
    let mut wildcard = input.clone();
    wildcard.rows[0].effects[0].arguments[0] = StaticEffectArgInfo::Wildcard;
    assert_ne!(
        base.type_fingerprint(TypeInfoRef(1)).unwrap(),
        graph(wildcard).type_fingerprint(TypeInfoRef(1)).unwrap()
    );
    let mut changed_owner = input;
    changed_owner.binders[1].owner = GenericOwner::Declaration(key("other", DefinitionKind::Flow));
    assert_ne!(
        base.type_fingerprint(TypeInfoRef(1)).unwrap(),
        graph(changed_owner)
            .type_fingerprint(TypeInfoRef(1))
            .unwrap()
    );
}

#[test]
fn rejects_missing_references_wrong_binder_kinds_and_malformed_identities() {
    let mut cases = vec![];
    let mut missing = generic_flow();
    missing.types[0] = TypeInfo::Tuple(vec![TypeInfoRef(999)]);
    cases.push(missing);
    let mut missing = generic_flow();
    missing.rows[0].tail = Some(GenericParameterRef(999));
    cases.push(missing);
    let mut wrong_kind = generic_flow();
    wrong_kind.rows[0].tail = Some(GenericParameterRef(0));
    cases.push(wrong_kind);
    let mut wrong_kind = generic_flow();
    wrong_kind.types[0] = TypeInfo::Parameter(GenericParameterRef(1));
    cases.push(wrong_kind);
    let mut wrong_effect = generic_flow();
    wrong_effect.rows[0].effects[0].target =
        EffectTargetInfo::Effect(key("NotEffect", DefinitionKind::Type));
    cases.push(wrong_effect);
    let mut duplicate = generic_flow();
    duplicate.binders[1] = duplicate.binders[0].clone();
    cases.push(duplicate);
    let mut invalid_bound = generic_flow();
    invalid_bound.binders[0].bounds.push(SpecInfo {
        definition: key("NotSpec", DefinitionKind::Type),
        arguments: vec![],
    });
    cases.push(invalid_bound);
    for input in cases {
        assert!(DescriptorGraph::new(input, DescriptorLimits::default()).is_err());
    }
    let duplicate_fields = DescriptorGraphInput {
        types: vec![
            TypeInfo::Record(vec![
                FieldInfo {
                    name: "x".into(),
                    ty: TypeInfoRef(1),
                },
                FieldInfo {
                    name: "x".into(),
                    ty: TypeInfoRef(1),
                },
            ]),
            TypeInfo::Primitive(PrimitiveInfo::Bool),
        ],
        ..Default::default()
    };
    assert!(matches!(
        DescriptorGraph::new(duplicate_fields, DescriptorLimits::default()),
        Err(DescriptorError::DuplicateName { .. })
    ));
}

#[test]
fn transparent_alias_and_anonymous_structural_cycles_are_rejected() {
    for types in [
        vec![TypeInfo::Alias {
            definition: key("A", DefinitionKind::Type),
            parameters: vec![],
            target: TypeInfoRef(0),
        }],
        vec![
            TypeInfo::Alias {
                definition: key("A", DefinitionKind::Type),
                parameters: vec![],
                target: TypeInfoRef(1),
            },
            TypeInfo::Tuple(vec![TypeInfoRef(0)]),
        ],
        vec![TypeInfo::Record(vec![FieldInfo {
            name: "self".into(),
            ty: TypeInfoRef(0),
        }])],
    ] {
        assert!(matches!(
            DescriptorGraph::new(
                DescriptorGraphInput {
                    types,
                    ..Default::default()
                },
                DescriptorLimits::default()
            ),
            Err(DescriptorError::UnguardedCycle { .. })
        ));
    }
}

#[test]
fn validates_applied_constructor_kind_ownership_and_arity() {
    let mut input = tree();
    input.types.push(TypeInfo::Primitive(PrimitiveInfo::I32));
    input.types.push(TypeInfo::Applied {
        constructor: TypeInfoRef(0),
        arguments: vec![TypeInfoRef(2)],
    });
    assert!(DescriptorGraph::new(input.clone(), DescriptorLimits::default()).is_ok());
    input.types[3] = TypeInfo::Applied {
        constructor: TypeInfoRef(0),
        arguments: vec![],
    };
    assert!(DescriptorGraph::new(input.clone(), DescriptorLimits::default()).is_err());
    input.types[3] = TypeInfo::Applied {
        constructor: TypeInfoRef(2),
        arguments: vec![],
    };
    assert!(DescriptorGraph::new(input, DescriptorLimits::default()).is_err());
    let mut owner = tree();
    owner.binders[0].owner = GenericOwner::Declaration(key("Other", DefinitionKind::Type));
    assert!(DescriptorGraph::new(owner, DescriptorLimits::default()).is_err());
}

#[test]
fn node_edge_text_and_expansion_budgets_apply_to_the_whole_graph() {
    let input = generic_flow();
    for limits in [
        DescriptorLimits {
            max_nodes: 4,
            ..Default::default()
        },
        DescriptorLimits {
            max_edges: 4,
            ..Default::default()
        },
        DescriptorLimits {
            max_text_bytes: 4,
            ..Default::default()
        },
        DescriptorLimits {
            max_expansion_depth: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            DescriptorGraph::new(input.clone(), limits),
            Err(DescriptorError::Limit { .. })
        ));
    }
    let mut types = vec![TypeInfo::Primitive(PrimitiveInfo::Unit)];
    for i in 0..10_000 {
        types.push(TypeInfo::Tuple(vec![TypeInfoRef(i)]));
    }
    assert!(matches!(
        DescriptorGraph::new(
            DescriptorGraphInput {
                types,
                ..Default::default()
            },
            DescriptorLimits::default()
        ),
        Err(DescriptorError::Limit {
            resource: "expansion depth",
            ..
        })
    ));
}

#[test]
fn copied_graphs_share_one_backing_and_do_not_retain_previous_graphs() {
    let graph = graph(tree());
    let base = MemoryFootprint::of(&graph);
    let copies = vec![graph.clone(); 100];
    assert!(std::ptr::eq(graph.data(), copies[99].data()));
    let total = MemoryFootprint::of(&copies);
    assert_eq!(
        total.estimated_bytes() - base.estimated_bytes(),
        (copies.capacity() * std::mem::size_of::<DescriptorGraph>()) as u64
    );
    assert_eq!(total.allocations.len(), base.allocations.len() + 1);
}

#[test]
fn recursive_descriptor_has_a_fixed_versioned_digest() {
    assert_eq!(
        graph(tree())
            .type_fingerprint(TypeInfoRef(0))
            .unwrap()
            .to_string(),
        "862559fc1d631ff4a7f69cb73ba47eeceaa468e35b47dd2186211704effcab09"
    );
}
