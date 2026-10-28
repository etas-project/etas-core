use super::{StdPrimitiveType, StdRecordField, StdType};

/// Canonical native value identities; unrelated source types cannot acquire these roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, etas_core::RetainedSize)]
pub enum RuntimeTypeRole {
    CommandResult,
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
        }
    }

    pub fn projected_fields(self) -> Vec<StdRecordField> {
        self.fields()
            .iter()
            .map(|field| StdRecordField::new(field.name, StdType::Primitive(field.ty)))
            .collect()
    }
}
