use super::*;

fn signature(kind: ActionArgKind, name: &str) -> ActionSignature {
    ActionSignature {
        path: vec!["library".into(), "Effect".into(), "action".into()],
        effect_args: vec![kind],
        selector_param_names: vec![name.into()],
        selector_defaults: vec![None],
        visibility: Visibility::Public,
        ..Default::default()
    }
}

fn decode(action: &ActionSignature) -> Result<ActionSignature, MetadataArtifactError> {
    let bytes = action_signature_to_proto(action).encode_to_vec();
    action_signature_from_proto(ProtoActionSignature::decode(bytes.as_slice()).unwrap())
}

#[test]
fn selector_codec_accepts_named_and_anonymous_non_type_selectors() {
    for kind in [
        ActionArgKind::MemoryPlace,
        ActionArgKind::StringPattern,
        ActionArgKind::StaticResourcePath {
            ty: "BrowserProfile".into(),
        },
    ] {
        for name in ["", "resource"] {
            let action = signature(kind.clone(), name);
            validate_model_action(&action).unwrap();
            let decoded = decode(&action).unwrap();
            assert_eq!(decoded, action);
        }
    }
}

fn rejected_on_both_boundaries(action: &ActionSignature, expected: &str) {
    let encoded = validate_model_action(action).unwrap_err();
    // Bypass the encoder gate to simulate untrusted input at the decoder.
    let decoded = decode(action).unwrap_err();
    assert!(encoded.to_string().contains(expected), "{encoded}");
    assert!(decoded.to_string().contains(expected), "{decoded}");
}

#[test]
fn selector_codec_requires_declared_type_parameter_identity_and_kind() {
    for name in ["", "Missing"] {
        rejected_on_both_boundaries(&signature(ActionArgKind::Type, name), "compatible generic");
    }
    let mut action = signature(ActionArgKind::Type, "T");
    action.generic_params.push(GenericParam {
        name: "T".into(),
        kind: GenericParamKind::Effect,
        bounds: Vec::new(),
    });
    rejected_on_both_boundaries(&action, "compatible generic");
    action.generic_params[0].kind = GenericParamKind::Type;
    validate_model_action(&action).unwrap();
    assert_eq!(decode(&action).unwrap(), action);
    action.generic_params.push(action.generic_params[0].clone());
    rejected_on_both_boundaries(&action, "duplicate generic");
}

#[test]
fn selector_codec_keeps_arity_and_default_kind_fail_closed() {
    let mut action = signature(ActionArgKind::MemoryPlace, "");
    action.selector_param_names.clear();
    rejected_on_both_boundaries(&action, "selector_param_names length");
    action.selector_param_names.push(String::new());
    action.selector_defaults.clear();
    rejected_on_both_boundaries(&action, "selector_defaults length");
    action.selector_defaults.push(Some(EffectArg {
        kind: EffectArgKind::String,
        value: "not a memory place".into(),
        ..Default::default()
    }));
    rejected_on_both_boundaries(&action, "does not match selector kind");
}
