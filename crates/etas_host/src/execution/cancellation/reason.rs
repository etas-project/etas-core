use super::super::ScopeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CancellationReason {
    Requested,
    Interrupt,
    Terminate,
    OwnerDropped,
    ChildFailed,
    Deadline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CancellationCause {
    origin: ScopeId,
    reason: CancellationReason,
}

impl CancellationCause {
    pub(crate) fn new(origin: ScopeId, reason: CancellationReason) -> Self {
        Self { origin, reason }
    }

    pub fn origin(&self) -> ScopeId {
        self.origin
    }
    pub fn reason(&self) -> &CancellationReason {
        &self.reason
    }
}
