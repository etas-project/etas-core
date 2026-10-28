use serde::{Serialize, ser::SerializeStruct};
use std::sync::Arc;

use super::{
    CodeFingerprint, DefinitionKey, DefinitionKind, IDENTITY_SCHEMA_VERSION, IdentityError,
    PackageKey,
    canonical::CanonicalEncoder,
    definition::{validate_path, validate_schema},
};

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, etas_core::RetainedSize, serde::Deserialize,
)]
#[serde(try_from = "WireImplementation")]
pub struct ImplementationKey(Arc<ImplementationIdentity>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, etas_core::RetainedSize)]
struct ImplementationIdentity {
    package: PackageKey,
    module: Arc<[String]>,
    subject: DefinitionKey,
    selected_spec: Option<DefinitionKey>,
    header: CodeFingerprint,
}

impl ImplementationKey {
    pub fn new(
        package: PackageKey,
        module: Vec<String>,
        subject: DefinitionKey,
        selected_spec: Option<DefinitionKey>,
        header: CodeFingerprint,
    ) -> Result<Self, IdentityError> {
        validate_path(&module, "implementation module")?;
        if let Some(spec) = &selected_spec
            && spec.kind() != DefinitionKind::Spec
        {
            return Err(IdentityError::NotSpec(spec.kind()));
        }
        Ok(Self(Arc::new(ImplementationIdentity {
            package,
            module: module.into(),
            subject,
            selected_spec,
            header,
        })))
    }
    pub fn package(&self) -> PackageKey {
        self.0.package
    }
    pub fn module(&self) -> &[String] {
        &self.0.module
    }
    pub fn subject(&self) -> &DefinitionKey {
        &self.0.subject
    }
    pub fn selected_spec(&self) -> Option<&DefinitionKey> {
        self.0.selected_spec.as_ref()
    }
    /// Header identity excludes the editable method body.
    pub fn header(&self) -> CodeFingerprint {
        self.0.header
    }
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.implementation.v1");
        encoder.fingerprint(self.package().fingerprint());
        encoder.path(self.module());
        encoder.fingerprint(self.subject().fingerprint());
        match self.selected_spec() {
            Some(spec) => {
                encoder.tag(1);
                encoder.fingerprint(spec.fingerprint());
            }
            None => encoder.tag(0),
        }
        encoder.fingerprint(self.header());
        encoder.finish()
    }
}

impl Serialize for ImplementationKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct("ImplementationKey", 6)?;
        record.serialize_field("schema", &IDENTITY_SCHEMA_VERSION)?;
        record.serialize_field("package", &self.package())?;
        record.serialize_field("module", self.module())?;
        record.serialize_field("subject", self.subject())?;
        record.serialize_field("selected_spec", &self.selected_spec())?;
        record.serialize_field("header", &self.header())?;
        record.end()
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireImplementation {
    schema: u32,
    package: PackageKey,
    module: Vec<String>,
    subject: DefinitionKey,
    selected_spec: Option<DefinitionKey>,
    header: CodeFingerprint,
}

impl TryFrom<WireImplementation> for ImplementationKey {
    type Error = IdentityError;
    fn try_from(value: WireImplementation) -> Result<Self, Self::Error> {
        validate_schema(value.schema)?;
        Self::new(
            value.package,
            value.module,
            value.subject,
            value.selected_spec,
            value.header,
        )
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, etas_core::RetainedSize, serde::Deserialize,
)]
#[serde(try_from = "WireMethod")]
pub struct MethodId(Arc<MethodIdentity>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, etas_core::RetainedSize)]
struct MethodIdentity {
    implementation: ImplementationKey,
    name: String,
}

impl MethodId {
    pub fn new(implementation: ImplementationKey, name: String) -> Result<Self, IdentityError> {
        validate_path(std::slice::from_ref(&name), "method")?;
        Ok(Self(Arc::new(MethodIdentity {
            implementation,
            name,
        })))
    }
    pub fn implementation(&self) -> &ImplementationKey {
        &self.0.implementation
    }
    pub fn name(&self) -> &str {
        &self.0.name
    }
    pub fn select_spec(&self, spec: DefinitionKey) -> Result<Self, IdentityError> {
        let owner = self.implementation();
        Self::new(
            ImplementationKey::new(
                owner.package(),
                owner.module().to_vec(),
                owner.subject().clone(),
                Some(spec),
                owner.header(),
            )?,
            self.name().into(),
        )
    }
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.method.v1");
        encoder.fingerprint(self.implementation().fingerprint());
        encoder.bytes(self.name().as_bytes());
        encoder.finish()
    }
}

impl Serialize for MethodId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct("MethodId", 3)?;
        record.serialize_field("schema", &IDENTITY_SCHEMA_VERSION)?;
        record.serialize_field("implementation", self.implementation())?;
        record.serialize_field("name", self.name())?;
        record.end()
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireMethod {
    schema: u32,
    implementation: ImplementationKey,
    name: String,
}

impl TryFrom<WireMethod> for MethodId {
    type Error = IdentityError;
    fn try_from(value: WireMethod) -> Result<Self, Self::Error> {
        validate_schema(value.schema)?;
        Self::new(value.implementation, value.name)
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, etas_core::RetainedSize, serde::Deserialize,
)]
#[serde(try_from = "WireCallable")]
pub struct CallableId(CallableIdentity);

pub enum CallableIdentityRef<'a> {
    Declaration(&'a DefinitionKey),
    Method(&'a MethodId),
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, etas_core::RetainedSize,
)]
enum CallableIdentity {
    Declaration(DefinitionKey),
    Method(MethodId),
}

#[derive(serde::Deserialize)]
enum WireCallable {
    Declaration(DefinitionKey),
    Method(MethodId),
}

impl CallableId {
    pub fn identity_ref(&self) -> CallableIdentityRef<'_> {
        match &self.0 {
            CallableIdentity::Declaration(key) => CallableIdentityRef::Declaration(key),
            CallableIdentity::Method(method) => CallableIdentityRef::Method(method),
        }
    }
    pub fn declaration(definition: DefinitionKey) -> Result<Self, IdentityError> {
        if !definition.kind().is_callable() {
            return Err(IdentityError::NotCallable(definition.kind()));
        }
        Ok(Self(CallableIdentity::Declaration(definition)))
    }
    pub fn method(method: MethodId) -> Self {
        Self(CallableIdentity::Method(method))
    }
    pub fn as_declaration(&self) -> Option<&DefinitionKey> {
        match &self.0 {
            CallableIdentity::Declaration(value) => Some(value),
            CallableIdentity::Method(_) => None,
        }
    }
    pub fn as_method(&self) -> Option<&MethodId> {
        match &self.0 {
            CallableIdentity::Method(value) => Some(value),
            CallableIdentity::Declaration(_) => None,
        }
    }
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.callable.v1");
        match &self.0 {
            CallableIdentity::Declaration(definition) => {
                encoder.tag(1);
                encoder.fingerprint(definition.fingerprint());
            }
            CallableIdentity::Method(method) => {
                encoder.tag(2);
                encoder.fingerprint(method.fingerprint());
            }
        }
        encoder.finish()
    }
}

impl Serialize for CallableId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl TryFrom<WireCallable> for CallableId {
    type Error = IdentityError;
    fn try_from(value: WireCallable) -> Result<Self, Self::Error> {
        match value {
            WireCallable::Declaration(definition) => Self::declaration(definition),
            WireCallable::Method(method) => Ok(Self::method(method)),
        }
    }
}
