#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum LoweringHint {
    None,
    PureBuiltin,
    RuntimeCall,
    Task(StdTaskIntrinsic),
    Collection(super::StdCollectionIntrinsic),
    HostBoundary,
    ApprovalBoundary,
    ErrorRaise,
    ForwardAction {
        stable_id: u32,
    },
    DefaultAction {
        stable_id: u32,
        payload: Vec<usize>,
    },
    ActionReducer {
        stable_id: u32,
        payload: Vec<usize>,
        reducer: StdActionReducer,
    },
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum StdTaskIntrinsic {
    Run,
    Start,
    Await,
    Requeue,
    Park,
    JoinTuple,
    JoinArray,
    TryJoin,
    Collect,
    Race,
    MapConcurrent,
    MapConcurrentCollect,
}

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub enum StdActionReducer {
    BoundedDirectoryList {
        entry_limit_parameter: usize,
        byte_limit_parameter: usize,
    },
    BoundedFileRead {
        limit_parameter: usize,
    },
    ReadUntilLimit {
        limit_parameter: usize,
        timeout_parameter: usize,
    },
}

impl LoweringHint {
    pub fn default_action(stable_id: u32, payload: &[usize]) -> Self {
        Self::DefaultAction {
            stable_id,
            payload: payload.to_vec(),
        }
    }
}
