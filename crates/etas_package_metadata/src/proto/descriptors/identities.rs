use etas_code::{CallableId, CodeFingerprint, DefinitionKey, ImplementationKey, MethodId};

use super::{
    super::{origins, schema::ProtoDefinitionKey},
    invalid, required, schema as p,
};
use crate::MetadataArtifactError as Error;

pub(super) fn encode_key(key: &DefinitionKey) -> ProtoDefinitionKey {
    ProtoDefinitionKey {
        package: key.package().fingerprint().as_bytes().to_vec(),
        module: key.module().to_vec(),
        declaration: key.declaration().to_vec(),
        kind: origins::kind_to_proto(key.kind()),
    }
}

pub(super) fn decode_key(key: ProtoDefinitionKey) -> Result<DefinitionKey, Error> {
    DefinitionKey::new(
        origins::package_key(key.package)?,
        key.module,
        key.declaration,
        origins::kind_from_proto(key.kind)?,
    )
    .map_err(|error| invalid(error.to_string()))
}

pub(super) fn encode_method(method: &MethodId) -> p::Method {
    let implementation = method.implementation();
    p::Method {
        package: implementation.package().fingerprint().as_bytes().to_vec(),
        module: implementation.module().to_vec(),
        subject: Some(encode_key(implementation.subject())),
        selected_spec: implementation.selected_spec().map(encode_key),
        header: implementation.header().as_bytes().to_vec(),
        name: method.name().into(),
    }
}

pub(super) fn decode_method(method: p::Method) -> Result<MethodId, Error> {
    let header: [u8; 32] = method
        .header
        .try_into()
        .map_err(|_| invalid("implementation header fingerprint must contain 32 bytes"))?;
    let implementation = ImplementationKey::new(
        origins::package_key(method.package)?,
        method.module,
        decode_key(required(method.subject, "method subject")?)?,
        method.selected_spec.map(decode_key).transpose()?,
        CodeFingerprint::from_bytes(header),
    )
    .map_err(|error| invalid(error.to_string()))?;
    MethodId::new(implementation, method.name).map_err(|error| invalid(error.to_string()))
}

pub(super) fn encode_callable(callable: &CallableId) -> p::Callable {
    let kind = match callable.identity_ref() {
        etas_code::identity::CallableIdentityRef::Declaration(key) => {
            p::callable::Kind::Declaration(encode_key(key))
        }
        etas_code::identity::CallableIdentityRef::Method(method) => {
            p::callable::Kind::Method(encode_method(method))
        }
    };
    p::Callable { kind: Some(kind) }
}

pub(super) fn decode_callable(callable: p::Callable) -> Result<CallableId, Error> {
    match required(callable.kind, "callable identity")? {
        p::callable::Kind::Declaration(key) => {
            CallableId::declaration(decode_key(key)?).map_err(|error| invalid(error.to_string()))
        }
        p::callable::Kind::Method(method) => Ok(CallableId::method(decode_method(method)?)),
    }
}
