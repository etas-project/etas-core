use super::*;
use etas_code::syntax::{RETAINED_BODY_SOURCE_SCHEMA, RetainedBodyForm, RetainedBodySource};

fn with_bodies() -> DefinitionDescriptorSet {
    let mut set = snapshot_tests::with_snapshot();
    let headers = set.headers.as_mut().unwrap();
    let text = "return input;";
    let content = CodeFingerprint::of_bytes("etas.code.body-content.v1", text.as_bytes());
    let snapshot = &mut headers.snapshots.as_mut().unwrap().agents[0];
    snapshot.revision.content = content;
    headers.bodies = Some(DefinitionBodies {
        entries: vec![CallableBodyInfo {
            callable: snapshot.revision.definition.clone(),
            access: BodyAccess::Exported,
            source: RetainedBodySource {
                schema: RETAINED_BODY_SOURCE_SCHEMA,
                form: RetainedBodyForm::BlockContents,
                source_unit: CodeFingerprint::from_bytes([12; 32]),
                source_start: 20,
                source_length: 200,
                content,
                text: text.into(),
            },
        }],
    });
    set
}

#[test]
fn retained_body_envelopes_roundtrip_with_exact_source_and_syntax_identity() {
    let set = with_bodies();
    set.validate(limits()).unwrap();
    let wire = to_proto(&set);
    assert_eq!(wire.version, 6);
    let decoded =
        from_proto(ProtoDescriptorGraph::decode(wire.encode_to_vec().as_slice()).unwrap()).unwrap();
    assert_eq!(decoded, set);
    let body = &decoded
        .headers
        .as_ref()
        .unwrap()
        .bodies
        .as_ref()
        .unwrap()
        .entries[0];
    let identity = body.source.identity(body.callable.fingerprint()).unwrap();
    assert_eq!(identity.content(), body.source.content);
    let mut expression = body.source.clone();
    expression.form = RetainedBodyForm::Expression;
    assert_ne!(
        identity,
        expression.identity(body.callable.fingerprint()).unwrap()
    );
}

#[test]
fn malformed_body_envelopes_cannot_acquire_a_valid_metadata_graph() {
    for case in 0..12 {
        let mut wire = to_proto(&with_bodies());
        let headers = wire.headers.as_mut().unwrap();
        let bodies = headers.bodies.as_mut().unwrap();
        let body = &mut bodies.entries[0];
        match case {
            0 => body.syntax_schema += 1,
            1 => body.form = 0,
            2 => body.access = 0,
            3 => {
                body.content.pop();
            }
            4 => body.text.push('x'),
            5 => body.source_start = u64::MAX,
            6 => {
                body.callable = Some(identities::encode_callable(
                    &CallableId::declaration(key("secret", DefinitionKind::Flow)).unwrap(),
                ))
            }
            7 => {
                let duplicate = body.clone();
                bodies.entries.push(duplicate);
            }
            8 => wire.version = 3,
            9 => headers.snapshots = None,
            10 => {
                headers.snapshots.as_mut().unwrap().agents[0]
                    .revision
                    .as_mut()
                    .unwrap()
                    .content = vec![99; 32]
            }
            11 => body.source_length = 21,
            _ => unreachable!(),
        }
        assert!(from_proto(wire).is_err(), "case {case}");
    }
}

#[test]
fn retained_body_records_use_the_shared_header_and_artifact_budgets() {
    let set = with_bodies();
    let mut model_budget = super::super::super::budget::MetadataGraphBudget::default();
    validate_model(&set, &mut model_budget).unwrap();
    let mut wire_budget = super::super::super::budget::MetadataGraphBudget::default();
    validate_wire(&to_proto(&set), &mut wire_budget).unwrap();
    assert_eq!(model_budget.used(), wire_budget.used());
    let mut without_bodies = set.clone();
    without_bodies.headers.as_mut().unwrap().bodies = None;
    let units = without_bodies
        .headers
        .as_ref()
        .unwrap()
        .resource_units(limits())
        .unwrap();
    assert!(
        set.headers
            .as_ref()
            .unwrap()
            .resource_units(DescriptorLimits {
                max_nodes: units,
                ..limits()
            })
            .is_err()
    );
}

#[test]
fn retained_flow_body_must_match_its_published_revision_content() {
    let mut set = with_bodies();
    let headers = set.headers.as_mut().unwrap();
    let snapshot = &mut headers.snapshots.as_mut().unwrap().flows[0];
    let mut body = headers.bodies.as_ref().unwrap().entries[0].clone();
    body.callable = snapshot.revision.definition.clone();
    snapshot.revision.content = body.source.content;
    headers.bodies.as_mut().unwrap().entries.push(body);
    assert_eq!(from_proto(to_proto(&set)).unwrap(), set);

    set.headers
        .as_mut()
        .unwrap()
        .snapshots
        .as_mut()
        .unwrap()
        .flows[0]
        .revision
        .content = CodeFingerprint::from_bytes([99; 32]);
    assert!(from_proto(to_proto(&set)).is_err());
}

#[test]
fn retained_source_origin_uses_canonical_package_and_module_not_import_aliases() {
    use etas_code::identity::{DefinitionModuleOrigin, PackageDefinitionOrigins};
    let set = with_bodies();
    let mut metadata = crate::PublicMetadata {
        definition_origins: Some(PackageDefinitionOrigins {
            package: key("Worker", DefinitionKind::Agent).package(),
            modules: vec![DefinitionModuleOrigin {
                path: vec!["alias".into(), "library".into()],
                canonical_path: vec!["library".into()],
            }],
            declarations: vec![],
        }),
        definition_descriptors: Some(set),
        ..Default::default()
    };
    validate_public_bodies(&metadata).unwrap();
    metadata.definition_origins.as_mut().unwrap().modules[0].path[0] = "another_alias".into();
    validate_public_bodies(&metadata).unwrap();
    metadata.definition_origins.as_mut().unwrap().modules[0].canonical_path[0] =
        "wrong_module".into();
    assert!(validate_public_bodies(&metadata).is_err());
    metadata.definition_origins.as_mut().unwrap().modules[0].canonical_path[0] = "library".into();
    metadata.definition_origins.as_mut().unwrap().package =
        PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([44; 32]));
    assert!(validate_public_bodies(&metadata).is_err());
    metadata.definition_origins = None;
    assert!(validate_public_bodies(&metadata).is_err());
}
