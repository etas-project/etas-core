use super::StdType;

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ValueDecl {
    pub name: String,
    pub ty: StdType,
    pub runtime_role: Option<StdValueRole>,
}

impl ValueDecl {
    pub fn new(name: &str, ty: &str) -> Self {
        Self {
            name: name.to_owned(),
            ty: StdType::parse(ty),
            runtime_role: None,
        }
    }

    pub fn runtime(role: StdValueRole) -> Self {
        Self {
            name: role.value_name().into(),
            ty: StdType::Named(role.type_path().join(".")),
            runtime_role: Some(role),
        }
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
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
#[serde(rename_all = "kebab-case")]
pub enum StdValueRole {
    DefaultCommandSandbox,
    LogicalTraceLabel,
}

impl StdValueRole {
    pub fn type_path(self) -> &'static [&'static str] {
        match self {
            Self::DefaultCommandSandbox => &["std", "host", "sandbox", "SandboxProfile"],
            Self::LogicalTraceLabel => &["std", "runtime", "trace", "TraceLabel"],
        }
    }

    pub fn value_name(self) -> &'static str {
        match self {
            Self::DefaultCommandSandbox => "DefaultCommandSandbox",
            Self::LogicalTraceLabel => "Logical",
        }
    }
}
