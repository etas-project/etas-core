use super::{LoweringHint, StdIntrinsicId};

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub struct IntrinsicDescriptor {
    pub id: StdIntrinsicId,
    pub qualified_path: Vec<String>,
    pub purity: IntrinsicPurity,
    pub dispatch: IntrinsicDispatch,
    pub lowering: LoweringHint,
    pub latent_effect: IntrinsicLatentEffect,
    pub memory_access: IntrinsicMemoryAccess,
    pub runtime_requirement: IntrinsicRuntimeRequirement,
}

impl IntrinsicDescriptor {
    pub fn runtime(id: u32, path: &[&str], purity: IntrinsicPurity) -> Self {
        Self {
            id: StdIntrinsicId(id),
            qualified_path: path.iter().map(|segment| (*segment).into()).collect(),
            purity,
            dispatch: IntrinsicDispatch::Runtime,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: IntrinsicLatentEffect::None,
            memory_access: IntrinsicMemoryAccess::None,
            runtime_requirement: IntrinsicRuntimeRequirement::None,
        }
    }
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum IntrinsicPurity {
    Pure,
    Runtime,
    Host,
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum IntrinsicDispatch {
    PureKernel,
    Runtime,
    Host,
    LoweringOnly,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    etas_core::RetainedSize,
)]
pub enum IntrinsicLatentEffect {
    #[default]
    None,
    TransparentFirstArg,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    etas_core::RetainedSize,
)]
pub enum IntrinsicMemoryAccess {
    #[default]
    None,
    ReadFirstArgStore,
    WriteFirstArgStore,
    WriteFirstArgIntent,
    ReadWriteFirstArgStore,
}

#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    etas_core::RetainedSize,
)]
pub enum IntrinsicRuntimeRequirement {
    #[default]
    None,
    Checkpoint,
    TaskScheduler,
}
