#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct CallableValueBehavior {
    pub determinism: CallableValueDeterminism,
    #[serde(default)]
    pub independent_determinism: Option<CallableValueDeterminism>,
    pub host_requirements: Vec<CallableValueHostRequirement>,
    pub interpreter_features: Vec<CallableValueInterpreterFeature>,
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
    prost::Enumeration,
)]
#[repr(i32)]
pub enum CallableValueDeterminism {
    Deterministic = 0,
    RuntimeMediated = 1,
    NonDeterministic = 2,
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
    prost::Enumeration,
)]
#[repr(i32)]
pub enum CallableValueHostRequirement {
    Agentic = 0,
    ToolCall = 1,
    HostAuthority = 2,
    DurableMemory = 3,
    Approval = 4,
    Checkpoint = 5,
    Time = 6,
    Network = 7,
    Tcp = 8,
    Stream = 9,
    Tls = 10,
    Browser = 11,
    Console = 12,
    FileIO = 13,
    Command = 14,
    SecretAccess = 15,
    Compiler = 16,
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
    prost::Enumeration,
)]
#[repr(i32)]
pub enum CallableValueInterpreterFeature {
    EffectHandler = 0,
    Resume = 1,
    Retry = 2,
    Checkpoint = 3,
    WorkflowOrchestration = 4,
}
