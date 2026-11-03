use super::*;
use crate::{
    CallableValueBehavior, CallableValueDeterminism, CallableValueGraph, CallableValueNode,
    CallableValueParameter, CallableValueScope, CallableValueSource,
};
use etas_code::{
    CodeFingerprint, DefinitionKey, DefinitionKind, ImplementationHeader, ImplementationKey,
    MethodId, PackageKey,
};

fn published_method() -> PublishedMethod {
    let package = PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([3; 32]));
    let subject = DefinitionKey::new(
        package,
        vec!["api".into()],
        vec!["Packet".into()],
        DefinitionKind::Type,
    )
    .unwrap();
    let owner = ImplementationKey::from_header(
        package,
        vec!["api".into()],
        subject,
        None,
        &ImplementationHeader::new(Vec::new()).unwrap(),
    )
    .unwrap();
    PublishedMethod {
        self_type: Type {
            kind: TypeKind::Nominal,
            path: vec!["api".into(), "Packet".into()],
            ..Default::default()
        },
        receiver: MethodReceiver::Value { slot: 0 },
        owner_params: Vec::new(),
        identity: MethodId::new(owner, "encode".into()).unwrap(),
        callable: CallableValueGraph {
            sources: vec![CallableValueSource::Value(0)],
            values: vec![CallableValueNode {
                residual_checks: Default::default(),
                handler_requests: None,
                behavior: CallableValueBehavior {
                    independent_determinism: Default::default(),
                    determinism: CallableValueDeterminism::Deterministic,
                    host_requirements: Vec::new(),
                    interpreter_features: Vec::new(),
                },
                parent_scope: CallableValueScope::Root,
                generic_params: Vec::new(),
                generic_bindings: Vec::new(),
                param_names: vec!["self".into()],
                params: vec![Type {
                    kind: TypeKind::Nominal,
                    path: vec!["api".into(), "Packet".into()],
                    ..Default::default()
                }],
                output: Type {
                    kind: TypeKind::Primitive,
                    name: "i32".into(),
                    ..Default::default()
                },
                public_effects: Default::default(),
                requested_actions: Default::default(),
                handled_requested_actions: Default::default(),
                default_handler_effects: Default::default(),
                parameter_bindings: Vec::new(),
                returned_sources: None,
                action_trace: ActionTrace::Empty,
            }],
        },
    }
}

fn implementation() -> SpecImpl {
    let method = published_method();
    SpecImpl {
        self_type: method.self_type,
        spec: vec!["api".into(), "Encoder".into()],
        args: Vec::new(),
        methods: vec![method.identity],
    }
}

#[test]
fn implementation_method_identity_and_closed_behavior_graph_round_trip() {
    let expected = implementation();
    let bytes = spec_impl_to_proto(&expected).encode_to_vec();
    let decoded = spec_impl_from_proto(ProtoSpecImpl::decode(bytes.as_slice()).unwrap()).unwrap();
    assert_eq!(expected, decoded);
    assert_eq!(decoded.methods[0].name(), "encode");
    let expected = published_method();
    let decoded = methods::from_proto(
        ProtoPublishedMethod::decode(methods::to_proto(&expected).encode_to_vec().as_slice())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(decoded, expected);
    assert_eq!(decoded.root().unwrap().output.name, "i32");
}

#[test]
fn decoder_requires_both_method_identity_and_behavior_evidence() {
    for field in [0, 1] {
        let mut wire = methods::to_proto(&published_method());
        if field == 0 {
            wire.identity = None;
        } else {
            wire.callable = None;
        }
        assert!(methods::from_proto(wire).is_err());
    }
}

#[test]
fn inherent_and_associated_methods_preserve_distinct_owner_and_method_binders() {
    use crate::{CallableValueGenericBinding, GenericParam, GenericParamKind};
    for receiver in [
        MethodReceiver::Value { slot: 0 },
        MethodReceiver::Associated,
    ] {
        let mut method = published_method();
        method.receiver = receiver;
        method.owner_params = vec![GenericParam {
            name: "T".into(),
            kind: GenericParamKind::Type,
            bounds: Vec::new(),
        }];
        method.self_type = Type {
            kind: TypeKind::Applied,
            path: vec!["api".into(), "Packet".into()],
            children: vec![Type {
                kind: TypeKind::Var,
                name: "owner_T".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let root = &mut method.callable.values[0];
        root.generic_params = method.owner_params.clone();
        root.generic_bindings = vec![
            CallableValueGenericBinding {
                label: "owner_T".into(),
                scope: CallableValueScope::Root,
                position: 0,
            },
            CallableValueGenericBinding {
                label: "method_T".into(),
                scope: CallableValueScope::Value(0),
                position: 0,
            },
        ];
        root.params = vec![method.self_type.clone()];
        root.output = Type {
            kind: TypeKind::Tuple,
            children: vec![
                Type {
                    kind: TypeKind::Var,
                    name: "owner_T".into(),
                    ..Default::default()
                },
                Type {
                    kind: TypeKind::Var,
                    name: "method_T".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        method.validate().unwrap();
        let mut package = PackageMetadata {
            version: 1,
            package: PackageIdentity {
                name: "library".into(),
                version: "0.1.0".into(),
                edition: "2026".into(),
            },
            ..Default::default()
        };
        package.public_metadata.methods.push(method.clone());
        let bytes = package_metadata_to_proto(&package).encode_to_vec();
        let restored = crate::package_metadata_from_package_graph_payload(&bytes).unwrap();
        assert_eq!(restored.public_metadata.methods, [method]);
        assert!(
            crate::package_metadata_to_sections(&restored)
                .unwrap()
                .iter()
                .any(|section| section.kind == crate::MetadataSectionKind::TypeContracts)
        );
    }
}

#[test]
fn encode_and_decode_reject_unowned_or_missing_implementation_roots() {
    for kind in [0, 1, 2, 3] {
        let mut model = published_method();
        let method = &mut model;
        match kind {
            0 => method.callable.sources.clear(),
            1 => method.callable.sources[0] = CallableValueSource::Value(9),
            2 => {
                method.callable.sources[0] =
                    CallableValueSource::Parameter(CallableValueParameter {
                        scope: CallableValueScope::Root,
                        position: 0,
                    })
            }
            _ => {
                method.callable.values[0].returned_sources =
                    Some(vec![CallableValueSource::Parameter(
                        CallableValueParameter {
                            scope: CallableValueScope::Root,
                            position: 0,
                        },
                    )])
            }
        }
        let wire = methods::to_proto(&model);
        assert!(methods::from_proto(wire).is_err(), "case {kind}");
        let mut metadata = PackageMetadata::default();
        metadata.public_metadata.methods.push(model);
        assert!(
            validate_package_metadata_graph(&metadata).is_err(),
            "case {kind}"
        );
    }
}

#[test]
fn implementation_graphs_share_the_whole_artifact_node_budget() {
    let mut model = published_method();
    model.receiver = MethodReceiver::Associated;
    model.callable.values[0].params = vec![
        Type {
            kind: TypeKind::Primitive,
            name: "unit".into(),
            ..Default::default()
        };
        crate::MAX_METADATA_GRAPH_NODES / 2
    ];
    model.callable.values[0].param_names = (0..crate::MAX_METADATA_GRAPH_NODES / 2)
        .map(|index| format!("p{index}"))
        .collect();
    let mut metadata = PackageMetadata::default();
    let mut second = model.clone();
    second.identity =
        MethodId::new(model.identity.implementation().clone(), "second".into()).unwrap();
    metadata.public_metadata.methods = vec![model, second];
    assert!(
        validate_package_metadata_graph(&metadata)
            .unwrap_err()
            .to_string()
            .contains("node count")
    );
}

#[test]
fn method_contracts_round_trip_without_inventing_declaration_paths() {
    let implementation = implementation();
    let target = crate::CallableContractTarget::Method(implementation.methods[0].clone());
    let flow = CallableSpecSatisfaction {
        item: target.clone(),
        spec: vec!["api".into(), "Pure".into()],
        args: Vec::new(),
    };
    let trace = TraceSpecConformance {
        item: target,
        target: TraceSpecConformanceTarget::Named {
            definition: DefinitionKey::new(
                PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([3; 32])),
                vec!["api".into()],
                vec!["ReadOnly".into()],
                DefinitionKind::Spec,
            )
            .unwrap(),
            spec: vec!["api".into(), "ReadOnly".into()],
            args: Vec::new(),
        },
    };
    assert_eq!(
        callable_spec_satisfaction_from_proto(callable_spec_satisfaction_to_proto(&flow)).unwrap(),
        flow
    );
    assert_eq!(
        trace_spec_conformance_from_proto(trace_spec_conformance_to_proto(&trace)).unwrap(),
        trace
    );
    let mut package = PackageMetadata {
        version: 1,
        package: PackageIdentity {
            name: "library".into(),
            version: "0.1.0".into(),
            edition: "2026".into(),
        },
        ..Default::default()
    };
    package.public_metadata.spec_impls.push(implementation);
    package.public_metadata.methods.push(published_method());
    package
        .public_metadata
        .callable_spec_satisfactions
        .push(flow);
    package.public_metadata.trace_spec_conformances.push(trace);
    validate_package_metadata_graph(&package).unwrap();
    let sections = crate::package_metadata_to_sections(&package).unwrap();
    let bytes = crate::encode_metadata_artifact(
        &crate::MetadataArtifactHeader {
            artifact_schema_version: crate::ARTIFACT_SCHEMA_VERSION,
            compiler_version: "test".into(),
            package_id: "library".into(),
            package_version: "0.1.0".into(),
            source_payload_hash: "blake3:source".into(),
            manifest_hash: "blake3:manifest".into(),
            dependency_lock_hash: "blake3:lock".into(),
            created_target: "test".into(),
        },
        sections,
    )
    .unwrap();
    let (_, decoded) =
        crate::package_metadata_from_artifact(std::path::Path::new("library.etasmeta"), &bytes)
            .unwrap();
    assert_eq!(decoded, package);
    package.public_metadata.spec_impls.clear();
    package.public_metadata.methods.clear();
    assert!(
        validate_package_metadata_graph(&package)
            .unwrap_err()
            .to_string()
            .contains("published implementation owner")
    );
    let untrusted_wire = package_metadata_to_proto(&package).encode_to_vec();
    assert!(
        crate::package_metadata_from_package_graph_payload(&untrusted_wire)
            .unwrap_err()
            .to_string()
            .contains("published implementation owner")
    );
}

#[test]
fn contract_codec_rejects_missing_or_ambiguous_callable_identity() {
    let identity = implementation().methods.remove(0);
    for target in [
        None,
        Some(ProtoCallableContractTarget::default()),
        Some(ProtoCallableContractTarget {
            path: vec!["api".into(), "fake".into()],
            method: Some(super::super::descriptors::encode_method_identity(&identity)),
        }),
    ] {
        assert!(
            trace_spec_conformance_from_proto(ProtoTraceSpecConformance {
                item: target,
                target_kind: "inline".into(),
                ..Default::default()
            })
            .is_err()
        );
    }
}
