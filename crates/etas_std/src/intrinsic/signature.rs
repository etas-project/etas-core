use super::{LoweringHint, StdIntrinsicId};

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntrinsicPurity {
    Pure,
    Runtime,
    Host,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntrinsicDispatch {
    PureKernel,
    Runtime,
    Host,
    LoweringOnly,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IntrinsicLatentEffect {
    #[default]
    None,
    TransparentFirstArg,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IntrinsicMemoryAccess {
    #[default]
    None,
    ReadFirstArgStore,
    WriteFirstArgStore,
    ReadWriteFirstArgStore,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IntrinsicRuntimeRequirement {
    #[default]
    None,
    Checkpoint,
}
