use super::*;

pub(super) fn to_proto(method: &PublishedMethod) -> ProtoPublishedMethod {
    let (receiver, receiver_slot) = match method.receiver {
        MethodReceiver::Value { slot } => ("value", Some(slot)),
        MethodReceiver::Associated => ("associated", None),
        MethodReceiver::Component => ("component", None),
    };
    ProtoPublishedMethod {
        identity: Some(super::super::descriptors::encode_method_identity(
            &method.identity,
        )),
        self_type: Some(type_to_proto(&method.self_type)),
        receiver: receiver.into(),
        receiver_slot,
        owner_params: method
            .owner_params
            .iter()
            .map(generic_param_to_proto)
            .collect(),
        callable: Some(callable_values::to_proto(&method.callable)),
    }
}

pub(super) fn from_proto(
    method: ProtoPublishedMethod,
) -> Result<PublishedMethod, MetadataArtifactError> {
    let receiver = match (method.receiver.as_str(), method.receiver_slot) {
        ("value", Some(slot)) => MethodReceiver::Value { slot },
        ("associated", None) => MethodReceiver::Associated,
        ("component", None) => MethodReceiver::Component,
        _ => return Err(invalid_method("invalid published method receiver ABI")),
    };
    let method = PublishedMethod {
        identity: super::super::descriptors::decode_method_identity(required_message(
            method.identity,
            "published method identity",
        )?)?,
        self_type: type_from_proto(required_message(
            method.self_type,
            "published method owner type",
        )?)?,
        receiver,
        owner_params: method
            .owner_params
            .into_iter()
            .map(generic_param_from_proto)
            .collect::<Result<_, _>>()?,
        callable: callable_values::from_proto(required_message(
            method.callable,
            "published method behavior graph",
        )?)?,
    };
    method.validate()?;
    Ok(method)
}

fn invalid_method(reason: &str) -> MetadataArtifactError {
    MetadataArtifactError::invalid(crate::PACKAGE_METADATA_FILE, reason)
}
