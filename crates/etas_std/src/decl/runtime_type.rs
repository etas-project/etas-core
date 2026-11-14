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
    SessionId,
    SessionConfig,
    ContextPolicy,
    RetentionPolicy,
    Limit,
    Task,
    Scheduler,
    Continuation,
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum RuntimeFieldRole {
    CommandExitCode,
    CommandStdout,
    CommandStderr,
}

#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeField {
    pub name: &'static str,
    pub ty: StdPrimitiveType,
    pub role: RuntimeFieldRole,
}

impl RuntimeTypeRole {
    pub fn type_parameter_count(self) -> usize {
        match self {
            Self::Deque | Self::Queue | Self::Stack | Self::OrderedSet | Self::WorkspacePath => 1,
            Self::PriorityQueue | Self::OrderedMap | Self::MemoryWriteIntent | Self::Task => 2,
            Self::Continuation => 1,
            _ => 0,
        }
    }

    pub(crate) fn accepts_declaration(self, declaration: &super::TypeDecl) -> bool {
        let kind = match self {
            Self::ContextPolicy | Self::RetentionPolicy | Self::Limit => super::TypeDeclKind::Enum,
            _ => super::TypeDeclKind::Support,
        };
        let representation = match self {
            Self::SessionId => {
                declaration.representation == Some(StdType::Primitive(StdPrimitiveType::String))
            }
            Self::SessionConfig | Self::Conversation => {
                matches!(declaration.representation, Some(StdType::Record(_)))
            }
            _ => declaration.representation.is_none(),
        };
        let generic_shape = declaration.params.len() == self.type_parameter_count()
            && match self {
                Self::Task => matches!(
                    declaration.params.as_slice(),
                    [
                        super::StdGenericParam {
                            kind: super::StdGenericParamKind::Type,
                            ..
                        },
                        super::StdGenericParam {
                            kind: super::StdGenericParamKind::EffectRow,
                            ..
                        }
                    ]
                ),
                Self::Continuation => declaration
                    .params
                    .iter()
                    .all(|parameter| parameter.kind == super::StdGenericParamKind::Type),
                _ => declaration
                    .params
                    .iter()
                    .all(|parameter| parameter.kind == super::StdGenericParamKind::Type),
            };
        declaration.kind == kind
            && generic_shape
            && declaration.canonical_type.is_none()
            && declaration.code_role.is_none()
            && !declaration.derivable
            && declaration.projected_fields == self.projected_fields()
            && representation
            && (!matches!(
                self,
                Self::CommandResult | Self::Task | Self::Scheduler | Self::Continuation
            ) || declaration.construction == super::StdTypeConstruction::IntrinsicOnly)
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
