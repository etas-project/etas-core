use super::*;

fn method() -> SpecMethod {
    SpecMethod {
        name: "encode".into(),
        path: vec!["library".into(), "SnapshotEncoder".into(), "encode".into()],
        receiver_param: Some(0),
        signature: Some(CallableSignature {
            path: vec!["library".into(), "SnapshotEncoder".into(), "encode".into()],
            generic_params: vec![GenericParam {
                name: "Receiver".into(),
                kind: GenericParamKind::Type,
                bounds: Vec::new(),
            }],
            input: vec![Type {
                kind: TypeKind::Var,
                name: "Receiver".into(),
                ..Default::default()
            }],
            output: Some(Type {
                kind: TypeKind::Primitive,
                name: "i32".into(),
                ..Default::default()
            }),
            ..Default::default()
        }),
    }
}

#[test]
fn receiver_binding_roundtrips_by_declared_parameter_identity() {
    let original = method();
    let bytes = spec_method_to_proto(&original).encode_to_vec();
    let decoded =
        spec_method_from_proto(ProtoSpecMethod::decode(bytes.as_slice()).unwrap()).unwrap();
    assert_eq!(original, decoded);
}

#[test]
fn invalid_receiver_bindings_fail_at_encode_and_decode() {
    let mut invalid = vec![method(); 4];
    invalid[0].receiver_param = Some(9);
    invalid[1].signature = None;
    invalid[2].signature.as_mut().unwrap().generic_params[0].kind = GenericParamKind::Effect;
    invalid[3].signature.as_mut().unwrap().input[0].name = "Other".into();
    for method in invalid {
        assert!(crate::spec_receivers::validate(&method).is_err());
        assert!(spec_method_from_proto(spec_method_to_proto(&method)).is_err());
        let mut metadata = PackageMetadata::default();
        metadata
            .public_metadata
            .spec_signatures
            .push(SpecSignature {
                methods: vec![method],
                ..Default::default()
            });
        assert!(validate_package_metadata_graph(&metadata).is_err());
    }
}
