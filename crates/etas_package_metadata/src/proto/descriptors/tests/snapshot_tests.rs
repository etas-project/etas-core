use super::*;
use etas_code::{RevisionInputs, RevisionKey, report::*};

pub(super) fn with_snapshot() -> DefinitionDescriptorSet {
    let mut set = header_tests::with_method_selection();
    let headers = set.headers.as_mut().unwrap();
    let agent = &headers.agents[0];
    let flow = headers.callables[0].id.as_declaration().unwrap();
    let environment = CodeFingerprint::from_bytes([3; 32]);
    headers.snapshots = Some(DefinitionSnapshots {
        methods: vec![],
        flows: vec![DefinitionSnapshotInfo {
            identity: flow.clone(),
            revision: RevisionInputs {
                definition: headers.callables[0].id.clone(),
                content: CodeFingerprint::from_bytes([9; 32]),
                environment,
                contract: CodeFingerprint::from_bytes([10; 32]),
                compilation: CodeFingerprint::from_bytes([6; 32]),
                parent: None,
            },
            dependencies: DependencyInfo {
                environment,
                lexical_owner: flow.clone(),
                standard_package: key("std", DefinitionKind::Type).package(),
                modules: vec![ModuleOriginInfo {
                    module: flow.module().to_vec(),
                    package: flow.package(),
                }],
                bindings: vec![],
                artifacts: vec![],
            },
        }],
        agents: vec![DefinitionSnapshotInfo {
            identity: agent.identity.clone(),
            revision: RevisionInputs {
                definition: agent.default_entry.clone().unwrap(),
                content: CodeFingerprint::from_bytes([4; 32]),
                environment,
                contract: CodeFingerprint::from_bytes([5; 32]),
                compilation: CodeFingerprint::from_bytes([6; 32]),
                parent: Some(RevisionKey::from_canonical_fingerprint(
                    CodeFingerprint::from_bytes([7; 32]),
                )),
            },
            dependencies: DependencyInfo {
                environment,
                lexical_owner: agent.identity.clone(),
                standard_package: key("std", DefinitionKind::Type).package(),
                modules: vec![ModuleOriginInfo {
                    module: agent.identity.module().to_vec(),
                    package: agent.identity.package(),
                }],
                bindings: vec![DefinitionBindingInfo {
                    path: vec!["dependency".into(), "read".into()],
                    definition: key("read", DefinitionKind::Flow),
                }],
                artifacts: vec![DependencyArtifactInfo {
                    import_root: "dependency".into(),
                    fingerprint: "aa".repeat(32),
                }],
            },
        }],
    });
    set
}

#[test]
fn snapshot_coordinates_roundtrip_without_serializing_execution_evidence() {
    let set = with_snapshot();
    set.validate(limits()).unwrap();
    let wire = to_proto(&set);
    assert_eq!(wire.version, 6);
    let decoded =
        from_proto(ProtoDescriptorGraph::decode(wire.encode_to_vec().as_slice()).unwrap()).unwrap();
    assert_eq!(decoded, set);
    assert_eq!(fingerprint(&decoded), fingerprint(&set));
    let snapshot = &decoded
        .headers
        .as_ref()
        .unwrap()
        .snapshots
        .as_ref()
        .unwrap()
        .agents[0];
    assert_eq!(
        snapshot.revision_info().key(),
        RevisionKey::from_inputs(&snapshot.revision)
    );
    let flow = &decoded
        .headers
        .as_ref()
        .unwrap()
        .snapshots
        .as_ref()
        .unwrap()
        .flows[0];
    assert_eq!(flow.identity.kind(), DefinitionKind::Flow);
    assert_eq!(
        flow.revision_info().key(),
        RevisionKey::from_inputs(&flow.revision)
    );
}

#[test]
fn agent_only_snapshots_keep_their_explicit_schema_without_flow_coordinates() {
    let mut set = with_snapshot();
    let headers = set.headers.as_mut().unwrap();
    let removed = headers.callables.remove(0).id;
    headers.agents[0].method_info = None;
    headers.snapshots.as_mut().unwrap().flows.clear();
    set.declarations
        .retain(|binding| Some(&binding.definition) != removed.as_declaration());
    assert_eq!(to_proto(&set).version, 3);
    assert_eq!(from_proto(to_proto(&set)).unwrap(), set);
}

fn with_method_snapshots() -> DefinitionDescriptorSet {
    let mut set = with_snapshot();
    let headers = set.headers.as_mut().unwrap();
    let owner = headers.snapshots.as_ref().unwrap().agents[0].clone();
    let methods = headers
        .callables
        .iter()
        .filter(|callable| callable.id.as_method().is_some())
        .map(|callable| {
            owner
                .method_snapshot(
                    &callable.id,
                    if callable.id == owner.revision.definition {
                        owner.revision.content
                    } else {
                        CodeFingerprint::from_bytes([18; 32])
                    },
                )
                .unwrap()
        })
        .collect();
    headers.snapshots.as_mut().unwrap().methods = methods;
    set
}

#[test]
fn method_snapshots_roundtrip_without_becoming_flow_definitions() {
    let set = with_method_snapshots();
    assert_eq!(to_proto(&set).version, 7);
    let decoded = from_proto(to_proto(&set)).unwrap();
    assert_eq!(decoded, set);
    let snapshots = decoded
        .headers
        .as_ref()
        .unwrap()
        .snapshots
        .as_ref()
        .unwrap();
    assert_eq!(snapshots.methods.len(), 2);
    assert_eq!(snapshots.methods[0], snapshots.agents[0]);
    assert_ne!(
        snapshots.methods[1].revision_info().key(),
        snapshots.agents[0].revision_info().key()
    );
    let mut model_budget = super::super::super::budget::MetadataGraphBudget::default();
    validate_model(&set, &mut model_budget).unwrap();
    let mut wire_budget = super::super::super::budget::MetadataGraphBudget::default();
    validate_wire(&to_proto(&set), &mut wire_budget).unwrap();
    assert_eq!(model_budget.used(), wire_budget.used());
}

#[test]
fn malformed_method_coordinates_cannot_change_owner_body_or_revision_environment() {
    for case in 0..13 {
        let mut wire = to_proto(&with_method_snapshots());
        let snapshots = wire.headers.as_mut().unwrap().snapshots.as_mut().unwrap();
        match case {
            0 => {
                snapshots.methods.pop();
            }
            1 => snapshots.methods.push(snapshots.methods[0].clone()),
            2 => snapshots.methods.clear(),
            3 => {
                snapshots.methods[0].identity =
                    Some(identities::encode_key(&key("Other", DefinitionKind::Agent)))
            }
            4 => {
                snapshots.methods[0].revision.as_mut().unwrap().definition =
                    Some(identities::encode_callable(
                        &CallableId::declaration(key("visit", DefinitionKind::Flow)).unwrap(),
                    ))
            }
            5 => snapshots.methods[0].revision.as_mut().unwrap().content = vec![99; 32],
            6 => snapshots.methods[1].revision.as_mut().unwrap().contract = vec![99; 32],
            7 => snapshots.methods[1].revision.as_mut().unwrap().compilation = vec![99; 32],
            8 => snapshots.methods[1].revision.as_mut().unwrap().parent = Some(vec![99; 32]),
            9 => snapshots.methods[1]
                .dependencies
                .as_mut()
                .unwrap()
                .modules
                .clear(),
            10 => {
                snapshots.methods[1]
                    .dependencies
                    .as_mut()
                    .unwrap()
                    .environment = vec![99; 32];
                snapshots.methods[1].revision.as_mut().unwrap().environment = vec![99; 32];
            }
            11 => snapshots.agents[0].revision.as_mut().unwrap().parent = None,
            12 => wire.version = 6,
            _ => unreachable!(),
        }
        let wire = ProtoDescriptorGraph::decode(wire.encode_to_vec().as_slice()).unwrap();
        assert!(from_proto(wire).is_err(), "case {case}");
    }
}

#[test]
fn malformed_flow_snapshots_are_rejected_before_a_descriptor_graph_is_published() {
    for case in 0..11 {
        let mut wire = to_proto(&with_snapshot());
        let headers = wire.headers.as_mut().unwrap();
        let snapshots = headers.snapshots.as_mut().unwrap();
        let flow = &mut snapshots.flows[0];
        match case {
            0 => flow.identity = Some(identities::encode_key(&key("visit", DefinitionKind::Agent))),
            1 => {
                flow.revision.as_mut().unwrap().definition = Some(identities::encode_callable(
                    &CallableId::declaration(key("Other", DefinitionKind::Flow)).unwrap(),
                ))
            }
            2 => {
                flow.dependencies.as_mut().unwrap().lexical_owner =
                    Some(identities::encode_key(&key("Other", DefinitionKind::Flow)))
            }
            3 => flow.dependencies.as_mut().unwrap().modules.clear(),
            4 => flow.dependencies.as_mut().unwrap().environment = vec![42; 32],
            5 => flow.revision.as_mut().unwrap().content = vec![0; 31],
            6 => {
                let duplicate = flow.clone();
                snapshots.flows.push(duplicate);
            }
            7 => snapshots.flows.clear(),
            8 => wire.version = 3,
            9 => headers.snapshots = None,
            10 => headers.agents[0].method_info = None,
            _ => unreachable!(),
        }
        let wire = ProtoDescriptorGraph::decode(wire.encode_to_vec().as_slice()).unwrap();
        assert!(from_proto(wire).is_err(), "case {case}");
    }
}

#[test]
fn malformed_snapshot_records_are_rejected_by_the_binary_boundary() {
    for case in 0..13 {
        let mut wire = to_proto(&with_snapshot());
        let headers = wire.headers.as_mut().unwrap();
        let snapshots = headers.snapshots.as_mut().unwrap();
        let snapshot = &mut snapshots.agents[0];
        match case {
            0 => snapshot.revision.as_mut().unwrap().definition = None,
            1 => snapshot.dependencies = None,
            2 => snapshot.revision.as_mut().unwrap().parent = Some(vec![0; 31]),
            3 => snapshot.dependencies.as_mut().unwrap().standard_package = vec![0; 31],
            4 => snapshot.dependencies.as_mut().unwrap().environment = vec![42; 32],
            5 => {
                snapshot.dependencies.as_mut().unwrap().lexical_owner =
                    Some(identities::encode_key(&key("Other", DefinitionKind::Agent)))
            }
            6 => {
                let modules = &mut snapshot.dependencies.as_mut().unwrap().modules;
                modules.push(modules[0].clone());
            }
            7 => snapshot.dependencies.as_mut().unwrap().bindings[0].path = vec![],
            8 => {
                snapshot.dependencies.as_mut().unwrap().artifacts[0].fingerprint =
                    "not-a-fingerprint".into()
            }
            9 => snapshots.agents.clear(),
            10 => {
                let duplicate = snapshot.clone();
                snapshots.agents.push(duplicate);
            }
            11 => wire.version = 2,
            12 => headers.snapshots = None,
            _ => unreachable!(),
        }
        let wire = ProtoDescriptorGraph::decode(wire.encode_to_vec().as_slice()).unwrap();
        assert!(from_proto(wire).is_err(), "case {case}");
    }
}

#[test]
fn snapshot_dependencies_share_the_header_and_artifact_resource_budget() {
    let mut set = header_tests::with_headers();
    let baseline = set
        .headers
        .as_ref()
        .unwrap()
        .resource_units(limits())
        .unwrap();
    set.headers = with_snapshot().headers;
    let restricted = DescriptorLimits {
        max_nodes: baseline,
        ..limits()
    };
    assert!(
        set.headers
            .as_ref()
            .unwrap()
            .resource_units(restricted)
            .is_err()
    );
    let wire = to_proto(&set);
    let mut model_budget = super::super::super::budget::MetadataGraphBudget::default();
    validate_model(&set, &mut model_budget).unwrap();
    let mut wire_budget = super::super::super::budget::MetadataGraphBudget::default();
    validate_wire(&wire, &mut wire_budget).unwrap();
    assert_eq!(model_budget.used(), wire_budget.used());
}

#[test]
fn snapshot_module_identity_includes_the_package_not_just_the_path() {
    let mut set = with_snapshot();
    let dependencies = &mut set
        .headers
        .as_mut()
        .unwrap()
        .snapshots
        .as_mut()
        .unwrap()
        .agents[0]
        .dependencies;
    let original = &dependencies.modules[0];
    dependencies.modules.push(ModuleOriginInfo {
        module: original.module.clone(),
        package: PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([42; 32])),
    });
    set.validate(limits()).unwrap();
    assert_eq!(from_proto(to_proto(&set)).unwrap(), set);
}
