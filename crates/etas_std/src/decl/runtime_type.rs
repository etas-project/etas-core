use super::{StdPrimitiveType, StdRecordField, StdType};

/// Canonical native value identities; unrelated source types cannot acquire these roles.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum RuntimeTypeRole {
    Command,
    CommandResult,
    JsonValue,
    Conversation,
    Provenance,
    ModelResponse,
    Deque,
    Queue,
    Stack,
    PriorityQueue,
    OrderedMap,
    OrderedSet,
    WorkspacePath,
    MemoryWriteIntent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum RuntimeFieldRole {
    CommandExitCode,
    CommandStdout,
    CommandStderr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeField {
    pub name: &'static str,
    pub ty: StdPrimitiveType,
    pub role: RuntimeFieldRole,
}

impl RuntimeTypeRole {
    pub fn type_parameter_count(self) -> usize {
        match self {
            Self::Deque | Self::Queue | Self::Stack | Self::OrderedSet | Self::WorkspacePath => 1,
            Self::PriorityQueue | Self::OrderedMap | Self::MemoryWriteIntent => 2,
            _ => 0,
        }
    }

    pub(crate) fn accepts_declaration(self, declaration: &super::TypeDecl) -> bool {
        declaration.kind == super::TypeDeclKind::Support
            && declaration.params.len() == self.type_parameter_count()
            && declaration.canonical_type.is_none()
            && declaration.code_role.is_none()
            && !declaration.derivable
            && declaration.projected_fields == self.projected_fields()
            && (declaration.representation.is_none() || self == Self::Conversation)
            && (self != Self::CommandResult
                || declaration.construction == super::StdTypeConstruction::IntrinsicOnly)
    }

    pub fn fields(self) -> &'static [RuntimeField] {
        match self {
            Self::CommandResult => &[
                RuntimeField {
                    name: "exit_code",
                    ty: StdPrimitiveType::I32,
                    role: RuntimeFieldRole::CommandExitCode,
                },
                RuntimeField {
                    name: "stdout",
                    ty: StdPrimitiveType::Bytes,
                    role: RuntimeFieldRole::CommandStdout,
                },
                RuntimeField {
                    name: "stderr",
                    ty: StdPrimitiveType::Bytes,
                    role: RuntimeFieldRole::CommandStderr,
                },
            ],
            _ => &[],
        }
    }

    pub fn projected_fields(self) -> Vec<StdRecordField> {
        self.fields()
            .iter()
            .map(|field| StdRecordField::new(field.name, StdType::Primitive(field.ty)))
            .collect()
    }
}
