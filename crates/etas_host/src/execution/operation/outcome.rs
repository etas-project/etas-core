use super::super::ScopeId;
use crate::{HostError, HostRequestId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub(crate) u64);

impl OperationId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// External evidence is independent of local resource termination and stop cause.
#[derive(Clone, Debug, PartialEq)]
pub enum ExternalOutcome {
    NotDispatched,
    Confirmed,
    Partial { completed_units: u64 },
    Failed(HostError),
    Unknown,
    StorageWrite(crate::storage::outcome::StorageWriteEvidence),
}

impl ExternalOutcome {
    pub fn is_uncertain(&self) -> bool {
        matches!(
            self,
            Self::Unknown
                | Self::Partial { .. }
                | Self::StorageWrite(crate::storage::outcome::StorageWriteEvidence {
                    status: crate::storage::outcome::CommitStatus::Unknown,
                    ..
                })
        )
    }
}

#[derive(Clone, Debug)]
pub struct OperationReport {
    pub(crate) id: OperationId,
    pub(crate) scope: ScopeId,
    pub(crate) parent: Option<OperationId>,
    pub(crate) request: Option<HostRequestId>,
    pub(crate) dispatched: bool,
    pub(crate) outcome: Option<ExternalOutcome>,
    pub(crate) cleanup_errors: Vec<HostError>,
    pub(crate) owner_lost: bool,
    pub(crate) completed_units: Option<u64>,
}

impl OperationReport {
    pub fn completed_units(&self) -> Option<u64> {
        self.completed_units
    }
    pub fn id(&self) -> OperationId {
        self.id
    }
    pub fn scope(&self) -> ScopeId {
        self.scope
    }
    pub fn parent(&self) -> Option<OperationId> {
        self.parent
    }
    pub fn request(&self) -> Option<HostRequestId> {
        self.request
    }
    pub fn dispatched(&self) -> bool {
        self.dispatched
    }
    pub fn outcome(&self) -> Option<&ExternalOutcome> {
        self.outcome.as_ref()
    }
    pub fn cleanup_errors(&self) -> &[HostError] {
        &self.cleanup_errors
    }
    pub fn owner_lost(&self) -> bool {
        self.owner_lost
    }
}
