use etas_code::{
    CallableId, CodeFingerprint, DefinitionKey, DefinitionKind, ImplementationHeader,
    ImplementationKey, MethodId, PackageKey, descriptor::*,
};

fn key(name: &str, kind: DefinitionKind) -> DefinitionKey {
    DefinitionKey::new(
        PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([6; 32])),
        vec!["app".into()],
        vec![name.into()],
        kind,
    )
    .unwrap()
}

fn method(name: &str) -> CallableId {
    let owner = key("Worker", DefinitionKind::Agent);
    let implementation = ImplementationKey::from_header(
        owner.package(),
        owner.module().to_vec(),
        owner,
        None,
        &ImplementationHeader::new(Vec::new()).unwrap(),
    )
    .unwrap();
    CallableId::method(MethodId::new(implementation, name.into()).unwrap())
}

fn selector(argument: StaticEffectArgInfo) -> DescriptorGraph {
    DescriptorGraph::new(
        DescriptorGraphInput {
            rows: vec![EffectRowInfo {
                effects: vec![EffectInfo {
                    target: EffectTargetInfo::Action {
                        effect: key("Agentic", DefinitionKind::Effect),
                        name: "infer".into(),
                    },
                    arguments: vec![argument],
                }],
                tail: None,
            }],
            ..Default::default()
        },
        DescriptorLimits::default(),
    )
    .unwrap()
}

#[test]
fn callable_selectors_preserve_method_identity_and_do_not_alias_resource_keys() {
    let run = selector(StaticEffectArgInfo::Callable(method("run")));
    let other = selector(StaticEffectArgInfo::Callable(method("explain")));
    let resource = selector(StaticEffectArgInfo::Resource(key(
        "Worker",
        DefinitionKind::Agent,
    )));
    let fingerprint = |graph: &DescriptorGraph| graph.row_fingerprint(EffectRowInfoRef(0)).unwrap();
    assert_ne!(fingerprint(&run), fingerprint(&other));
    assert_ne!(fingerprint(&run), fingerprint(&resource));
    let (projected, roots) = run
        .project_roots(&[DescriptorNode::Row(EffectRowInfoRef(0))])
        .unwrap();
    assert_eq!(roots, vec![DescriptorNode::Row(EffectRowInfoRef(0))]);
    assert_eq!(fingerprint(&run), fingerprint(&projected));
}
