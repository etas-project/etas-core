use super::super::{CancellationCause, OperationReport, ScopeId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeOutcome {
    Completed,
    Failed,
    Cancelled(CancellationCause),
}

#[derive(Clone, Debug)]
pub struct TerminationReport {
    pub(crate) scope: ScopeId,
    pub(crate) outcome: ScopeOutcome,
    pub(crate) causes: Vec<CancellationCause>,
    pub(crate) operations: Vec<OperationReport>,
}

impl TerminationReport {
    pub fn scope(&self) -> ScopeId {
        self.scope
    }
    pub fn outcome(&self) -> &ScopeOutcome {
        &self.outcome
    }
    pub fn causes(&self) -> &[CancellationCause] {
        &self.causes
    }
    pub fn operations(&self) -> &[OperationReport] {
        &self.operations
    }
}

#[derive(Clone, Debug)]
pub struct PendingWork {
    pub(crate) scopes: Vec<ScopeId>,
    pub(crate) operations: Vec<OperationReport>,
}

impl PendingWork {
    pub fn scopes(&self) -> &[ScopeId] {
        &self.scopes
    }
    pub fn operations(&self) -> &[OperationReport] {
        &self.operations
    }
}
