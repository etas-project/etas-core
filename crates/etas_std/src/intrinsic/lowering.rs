#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum LoweringHint {
    None,
    PureBuiltin,
    RuntimeCall,
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

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum StdActionReducer {
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
