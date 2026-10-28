use etas_code::{CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, descriptor::*};

fn key(name: &str, kind: DefinitionKind) -> DefinitionKey {
    DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
        vec!["app".into()],
        vec![name.into()],
        kind,
    )
    .unwrap()
}

fn fixture() -> (DescriptorGraph, TraceConstraintInfo) {
    let target = EffectTargetInfo::Action {
        effect: key("Probe", DefinitionKind::Effect),
        name: "read".into(),
    };
    let graph = DescriptorGraph::new(
        DescriptorGraphInput {
            types: vec![
                TypeInfo::Record(vec![FieldInfo {
                    name: "private_unrelated_field".into(),
                    ty: TypeInfoRef(1),
                }]),
                TypeInfo::Primitive(PrimitiveInfo::I32),
            ],
            rows: vec![
                EffectRowInfo {
                    effects: Vec::new(),
                    tail: None,
                },
                EffectRowInfo {
                    effects: vec![EffectInfo {
                        target: target.clone(),
                        arguments: vec![StaticEffectArgInfo::Type(TypeInfoRef(1))],
                    }],
                    tail: None,
                },
            ],
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    let pattern = TracePatternInfo {
        row: EffectRowInfoRef(1),
        bounds: vec![TraceSelectorBoundInfo {
            target,
            argument: 0,
            spec: SpecInfo {
                definition: key("Allowed", DefinitionKind::Spec),
                arguments: vec![TypeInfoRef(1)],
            },
        }],
    };
    (
        graph,
        TraceConstraintInfo {
            alternatives: vec![
                vec![
                    TraceRuleInfo::Deny(pattern.clone()),
                    TraceRuleInfo::Limit { count: 3 },
                ],
                vec![
                    TraceRuleInfo::Allow(pattern.clone()),
                    TraceRuleInfo::RequireBefore {
                        guard: pattern.clone(),
                        target: pattern.clone(),
                    },
                    TraceRuleInfo::RequireAfter {
                        target: pattern.clone(),
                        obligation: pattern,
                    },
                ],
            ],
        },
    )
}

#[test]
fn trace_constraint_export_preserves_alternatives_bounds_and_root_scoping() {
    let (graph, constraint) = fixture();
    let fingerprint = constraint.fingerprint(&graph).unwrap();
    let (projected, rebased) = constraint.project(&graph).unwrap();
    assert_eq!(
        rebased
            .alternatives
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(
        projected.data().types,
        [TypeInfo::Primitive(PrimitiveInfo::I32)]
    );
    assert_eq!(projected.data().rows.len(), 1);
    assert_eq!(fingerprint, rebased.fingerprint(&projected).unwrap());
    for pattern in rebased
        .alternatives
        .iter()
        .flatten()
        .flat_map(TraceRuleInfo::patterns)
    {
        assert_eq!(pattern.row, EffectRowInfoRef(0));
        assert_eq!(pattern.bounds[0].spec.arguments, [TypeInfoRef(0)]);
    }
    let mut changed = constraint.clone();
    changed.alternatives[0].push(TraceRuleInfo::Limit { count: 1 });
    assert_ne!(fingerprint, changed.fingerprint(&graph).unwrap());
    assert_ne!(
        fingerprint,
        AttachedSpecInfo::Named(SpecInfo {
            definition: key("Allowed", DefinitionKind::Spec),
            arguments: Vec::new()
        })
        .fingerprint(&graph)
        .unwrap()
    );
}

#[test]
fn trace_constraint_rejects_missing_roots_and_inconsistent_bound_identities() {
    let (graph, constraint) = fixture();
    for invalid in 0..4 {
        let mut broken = constraint.clone();
        let TraceRuleInfo::Deny(pattern) = &mut broken.alternatives[0][0] else {
            panic!("deny");
        };
        match invalid {
            0 => pattern.row = EffectRowInfoRef(9),
            1 => pattern.bounds[0].argument = 9,
            2 => pattern.bounds[0].spec.arguments[0] = TypeInfoRef(9),
            3 => pattern.bounds[0].spec.definition = key("value", DefinitionKind::Value),
            _ => unreachable!(),
        }
        assert!(broken.project(&graph).is_err(), "case {invalid}");
        assert!(broken.fingerprint(&graph).is_err(), "case {invalid}");
    }
}
