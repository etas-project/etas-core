use serde::{Serialize, ser::SerializeStruct};
use std::{fmt, sync::Arc};

use super::{CodeFingerprint, IDENTITY_SCHEMA_VERSION, canonical::CanonicalEncoder};

/// Supplied by the package owner from canonical origin and locked identity,
/// not an arena ID, import alias, display name or source-content hash alone.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct PackageKey(CodeFingerprint);

impl PackageKey {
    pub const fn from_canonical_fingerprint(fingerprint: CodeFingerprint) -> Self {
        Self(fingerprint)
    }
    pub const fn fingerprint(self) -> CodeFingerprint {
        self.0
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum DefinitionKind {
    Agent,
    Flow,
    Tool,
    Type,
    Effect,
    Spec,
    Value,
    EnumVariant,
    Requirement,
    Protocol,
}

impl DefinitionKind {
    pub(super) fn tag(self) -> u8 {
        match self {
            Self::Agent => 1,
            Self::Flow => 2,
            Self::Tool => 3,
            Self::Type => 4,
            Self::Effect => 5,
            Self::Spec => 6,
            Self::Value => 7,
            Self::EnumVariant => 8,
            Self::Requirement => 9,
            Self::Protocol => 10,
        }
    }
    pub fn is_callable(self) -> bool {
        matches!(self, Self::Agent | Self::Flow | Self::Tool)
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, etas_core::RetainedSize, serde::Deserialize,
)]
#[serde(try_from = "WireDefinition")]
pub struct DefinitionKey(Arc<DefinitionIdentity>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, etas_core::RetainedSize)]
struct DefinitionIdentity {
    package: PackageKey,
    module: Arc<[String]>,
    declaration: Arc<[String]>,
    kind: DefinitionKind,
}

impl DefinitionKey {
    pub fn new(
        package: PackageKey,
        module: Vec<String>,
        declaration: Vec<String>,
        kind: DefinitionKind,
    ) -> Result<Self, IdentityError> {
        validate_path(&module, "module")?;
        if declaration.is_empty() {
            return Err(IdentityError::EmptyDeclaration);
        }
        validate_path(&declaration, "declaration")?;
        Ok(Self(Arc::new(DefinitionIdentity {
            package,
            module: module.into(),
            declaration: declaration.into(),
            kind,
        })))
    }

    pub fn package(&self) -> PackageKey {
        self.0.package
    }
    pub fn module(&self) -> &[String] {
        &self.0.module
    }
    pub fn declaration(&self) -> &[String] {
        &self.0.declaration
    }
    pub fn kind(&self) -> DefinitionKind {
        self.0.kind
    }

    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.definition.v1");
        encoder.fingerprint(self.package().fingerprint());
        encoder.path(self.module());
        encoder.path(self.declaration());
        encoder.tag(self.kind().tag());
        encoder.finish()
    }
}

impl Serialize for DefinitionKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut record = serializer.serialize_struct("DefinitionKey", 5)?;
        record.serialize_field("schema", &IDENTITY_SCHEMA_VERSION)?;
        record.serialize_field("package", &self.package())?;
        record.serialize_field("module", self.module())?;
        record.serialize_field("declaration", self.declaration())?;
        record.serialize_field("kind", &self.kind())?;
        record.end()
    }
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct WireDefinition {
    schema: u32,
    package: PackageKey,
    module: Vec<String>,
    declaration: Vec<String>,
    kind: DefinitionKind,
}

impl TryFrom<WireDefinition> for DefinitionKey {
    type Error = IdentityError;
    fn try_from(value: WireDefinition) -> Result<Self, Self::Error> {
        validate_schema(value.schema)?;
        Self::new(value.package, value.module, value.declaration, value.kind)
    }
}

pub(super) fn validate_schema(schema: u32) -> Result<(), IdentityError> {
    if schema == IDENTITY_SCHEMA_VERSION {
        Ok(())
    } else {
        Err(IdentityError::UnsupportedSchema(schema))
    }
}

pub(super) fn validate_path(path: &[String], part: &'static str) -> Result<(), IdentityError> {
    for (index, segment) in path.iter().enumerate() {
        if segment.is_empty() || segment.contains('\0') {
            return Err(IdentityError::InvalidSegment { part, index });
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityError {
    EmptyDeclaration,
    InvalidSegment { part: &'static str, index: usize },
    UnsupportedSchema(u32),
    NotSpec(DefinitionKind),
    NotCallable(DefinitionKind),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDeclaration => {
                formatter.write_str("definition identity has no declaration path")
            }
            Self::InvalidSegment { part, index } => {
                write!(formatter, "invalid {part} identity segment {index}")
            }
            Self::UnsupportedSchema(schema) => {
                write!(formatter, "unsupported code identity schema {schema}")
            }
            Self::NotSpec(kind) => write!(formatter, "selected spec identity has kind {kind:?}"),
            Self::NotCallable(kind) => write!(
                formatter,
                "callable identity has non-callable kind {kind:?}"
            ),
        }
    }
}

impl std::error::Error for IdentityError {}
