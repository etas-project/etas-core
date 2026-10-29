use super::*;
use crate::{
    CallableValueBehavior, CallableValueDeterminism, CallableValueGraph, CallableValueNode,
    CallableValueParameter, CallableValueScope, CallableValueSource,
};
use etas_code::{
    CodeFingerprint, DefinitionKey, DefinitionKind, ImplementationHeader, ImplementationKey,
    MethodId, PackageKey,
};

fn implementation() -> SpecImpl {
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
    SpecImpl {
        self_type: Type {
            kind: TypeKind::Nominal,
            path: vec!["api".into(), "Packet".into()],
            ..Default::default()
        },
        spec: vec!["api".into(), "Encoder".into()],
        args: Vec::new(),
        methods: vec![SpecImplMethod {
            identity: MethodId::new(owner, "encode".into()).unwrap(),
            callable: CallableValueGraph {
                sources: vec![CallableValueSource::Value(0)],
                values: vec![CallableValueNode {
                    handler_requests: None,
                    behavior: CallableValueBehavior {
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
        }],
    }
}

#[test]
fn implementation_method_identity_and_closed_behavior_graph_round_trip() {
    let expected = implementation();
    let bytes = spec_impl_to_proto(&expected).encode_to_vec();
    let decoded = spec_impl_from_proto(ProtoSpecImpl::decode(bytes.as_slice()).unwrap()).unwrap();
    assert_eq!(expected, decoded);
    assert_eq!(decoded.methods[0].identity.name(), "encode");
    assert_eq!(decoded.methods[0].root().unwrap().output.name, "i32");
}

#[test]
fn decoder_requires_both_method_identity_and_behavior_evidence() {
    for field in [0, 1] {
        let mut wire = spec_impl_to_proto(&implementation());
        if field == 0 {
            wire.methods[0].identity = None;
        } else {
            wire.methods[0].callable = None;
        }
        assert!(spec_impl_from_proto(wire).is_err());
    }
}

#[test]
fn encode_and_decode_reject_unowned_or_missing_implementation_roots() {
    for kind in [0, 1, 2, 3] {
        let mut model = implementation();
        let method = &mut model.methods[0];
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
        let wire = spec_impl_to_proto(&model);
        assert!(spec_impl_from_proto(wire).is_err(), "case {kind}");
        let mut metadata = PackageMetadata::default();
        metadata.public_metadata.spec_impls.push(model);
        assert!(
            validate_package_metadata_graph(&metadata).is_err(),
            "case {kind}"
        );
    }
}

#[test]
fn implementation_graphs_share_the_whole_artifact_node_budget() {
    let mut model = implementation();
    model.methods[0].callable.values[0].params = vec![
        Type {
            kind: TypeKind::Primitive,
            name: "unit".into(),
            ..Default::default()
        };
        crate::MAX_METADATA_GRAPH_NODES / 2
    ];
    model.methods[0].callable.values[0].param_names = (0..crate::MAX_METADATA_GRAPH_NODES / 2)
        .map(|index| format!("p{index}"))
        .collect();
    let mut metadata = PackageMetadata::default();
    metadata.public_metadata.spec_impls = vec![model.clone(), model];
    assert!(
        validate_package_metadata_graph(&metadata)
            .unwrap_err()
            .to_string()
            .contains("node count")
    );
}
