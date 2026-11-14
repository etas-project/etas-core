use super::{StdSpecRef, StdType};

#[derive(
    serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, etas_core::RetainedSize,
)]
pub struct StdImplFact {
    pub self_type: StdType,
    pub spec: StdSpecRef,
}

impl StdImplFact {
    pub fn new(self_type: StdType, spec: StdSpecRef) -> Self {
        Self { self_type, spec }
    }
}
