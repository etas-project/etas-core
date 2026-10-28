use super::*;
use etas_code::{RevisionInputs, RevisionKey, report::*};

pub(super) fn with_snapshot() -> DefinitionDescriptorSet {
    let mut set = header_tests::with_headers();
    let headers = set.headers.as_mut().unwrap();
    let agent = &headers.agents[0];
    let environment = CodeFingerprint::from_bytes([3; 32]);
    headers.snapshots = Some(DefinitionSnapshots {
        agents: vec![AgentSnapshotInfo {
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
    assert_eq!(wire.version, 3);
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
