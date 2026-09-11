use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use super::super::{
    CancelSignal, CancelSource, CancellationCause, CancellationReason, ExternalOutcome,
    OperationContext, OperationId, OperationRegistration, OperationReport, PendingWork,
    ScopeOutcome, TerminationReport,
};
use super::{ScopeId, ScopeState};
use crate::{HostError, HostErrorCode, HostRequestId, TraceContext};

#[derive(Debug)]
struct Node {
    parent: Option<ScopeId>,
    children: Vec<ScopeId>,
    token: CancellationToken,
    state: ScopeState,
    body_done: bool,
    body_claimed: bool,
    causes: Vec<CancellationCause>,
    operations: BTreeMap<OperationId, OperationReport>,
    terminal: Option<TerminationReport>,
}

impl Node {
    fn new(parent: Option<ScopeId>, token: CancellationToken) -> Self {
        Self {
            parent,
            token,
            children: Vec::new(),
            state: ScopeState::Running,
            body_done: false,
            body_claimed: false,
            causes: Vec::new(),
            operations: BTreeMap::new(),
            terminal: None,
        }
    }
}

#[derive(Debug)]
struct Registry {
    nodes: Vec<Node>,
    next_operation: u64,
}

#[derive(Debug)]
struct Tree {
    registry: Mutex<Registry>,
    changed: watch::Sender<u64>,
}

/// All scopes share a short admission lock, never an I/O or cleanup lock.
/// The tree owns child state; handles own the tree, with no strong back-reference cycle.
#[derive(Clone, Debug)]
pub struct ExecutionScope {
    tree: Arc<Tree>,
    id: ScopeId,
}

pub(crate) fn invalid_state(message: &str) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, message)
}

impl Default for ExecutionScope {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionScope {
    pub fn new() -> Self {
        Self::create(false)
    }

    /// Construct an unpublished root with its engine body already reserved.
    pub fn new_owned() -> Self {
        Self::create(true)
    }

    fn create(body_claimed: bool) -> Self {
        let (changed, _) = watch::channel(0);
        let mut root = Node::new(None, CancellationToken::new());
        root.body_claimed = body_claimed;
        Self {
            tree: Arc::new(Tree {
                registry: Mutex::new(Registry {
                    nodes: vec![root],
                    next_operation: 0,
                }),
                changed,
            }),
            id: ScopeId(0),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, Registry>, HostError> {
        self.tree
            .registry
            .lock()
            .map_err(|_| invalid_state("execution registry lock poisoned"))
    }

    fn notify(&self) {
        self.tree
            .changed
            .send_modify(|version| *version = version.wrapping_add(1));
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.tree.changed.subscribe()
    }
    pub fn id(&self) -> ScopeId {
        self.id
    }
    /// Reserve the unique engine owner. A stopped-but-not-started invocation may
    /// still be claimed so its driver can publish cancellation without executing.
    pub fn claim_body(&self) -> Result<(), HostError> {
        let mut registry = self.lock()?;
        let node = &mut registry.nodes[self.id.0 as usize];
        if node.body_claimed || node.body_done {
            return Err(invalid_state("execution scope already has a body owner"));
        }
        node.body_claimed = true;
        Ok(())
    }
    pub fn cancel_source(&self) -> CancelSource {
        CancelSource::new(self.clone())
    }

    pub fn signal(&self) -> Result<CancelSignal, HostError> {
        let token = self.lock()?.nodes[self.id.0 as usize].token.clone();
        Ok(CancelSignal::new(self.clone(), token))
    }

    pub fn state(&self) -> Result<ScopeState, HostError> {
        Ok(self.lock()?.nodes[self.id.0 as usize].state)
    }

    pub(crate) fn cause(&self) -> Result<Option<CancellationCause>, HostError> {
        Ok(self.lock()?.nodes[self.id.0 as usize]
            .causes
            .first()
            .cloned())
    }

    pub fn child(&self) -> Result<Self, HostError> {
        let mut registry = self.lock()?;
        registry.check_admission(self.id, false)?;
        let id = ScopeId(registry.nodes.len() as u64);
        let token = registry.nodes[self.id.0 as usize].token.child_token();
        registry.nodes.push(Node::new(Some(self.id), token));
        registry.nodes[self.id.0 as usize].children.push(id);
        drop(registry);
        self.notify();
        Ok(Self {
            tree: self.tree.clone(),
            id,
        })
    }

    pub fn register(
        &self,
        parent: Option<OperationId>,
        request: Option<HostRequestId>,
        trace: TraceContext,
    ) -> Result<OperationRegistration, HostError> {
        let mut registry = self.lock()?;
        if let Some(parent) = parent {
            let owner = registry.nodes[self.id.0 as usize]
                .operations
                .get(&parent)
                .ok_or_else(|| invalid_state("operation parent is not owned by this scope"))?;
            if owner.outcome.is_some() || owner.owner_lost {
                return Err(invalid_state("operation parent is no longer active"));
            }
        }
        registry.check_admission(self.id, parent.is_some())?;
        let id = OperationId(registry.next_operation);
        registry.next_operation = registry
            .next_operation
            .checked_add(1)
            .ok_or_else(|| invalid_state("execution operation identity exhausted"))?;
        let node = &mut registry.nodes[self.id.0 as usize];
        let signal = CancelSignal::new(self.clone(), node.token.clone());
        node.operations.insert(
            id,
            OperationReport {
                id,
                scope: self.id,
                parent,
                request,
                dispatched: false,
                outcome: None,
                cleanup_errors: Vec::new(),
                owner_lost: false,
                completed_units: None,
            },
        );
        drop(registry);
        self.notify();
        Ok(OperationRegistration::new(
            self.clone(),
            OperationContext::new(self.id, id, parent, request, trace, signal),
        ))
    }

    pub(crate) fn begin_dispatch(&self, operation: OperationId) -> Result<(), HostError> {
        let mut registry = self.lock()?;
        registry.check_admission(self.id, true)?;
        let record = registry.operation_mut(self.id, operation)?;
        if record.dispatched || record.outcome.is_some() || record.owner_lost {
            return Err(invalid_state(
                "operation cannot be dispatched twice or without its owner",
            ));
        }
        record.dispatched = true;
        Ok(())
    }

    pub(crate) fn complete_operation(
        &self,
        operation: OperationId,
        outcome: ExternalOutcome,
        errors: Vec<HostError>,
    ) -> Result<(), HostError> {
        let mut registry = self.lock()?;
        let record = registry.operation_mut(self.id, operation)?;
        if record.outcome.is_some() || record.owner_lost {
            return Err(invalid_state("operation completion has no active owner"));
        }
        if !record.dispatched && outcome != ExternalOutcome::NotDispatched {
            return Err(invalid_state(
                "undispatched operation cannot publish external completion",
            ));
        }
        if record.dispatched && outcome == ExternalOutcome::NotDispatched {
            return Err(invalid_state(
                "dispatched operation cannot claim not dispatched",
            ));
        }
        record.outcome = Some(match (outcome, record.completed_units) {
            (ExternalOutcome::Unknown, Some(completed_units)) => {
                ExternalOutcome::Partial { completed_units }
            }
            (outcome, _) => outcome,
        });
        record.cleanup_errors.extend(errors);
        registry.settle(self.id);
        drop(registry);
        self.notify();
        Ok(())
    }

    pub(crate) fn record_cleanup_error(
        &self,
        operation: OperationId,
        error: HostError,
    ) -> Result<(), HostError> {
        let mut registry = self.lock()?;
        let record = registry.operation_mut(self.id, operation)?;
        if !record.dispatched || record.outcome.is_some() || record.owner_lost {
            return Err(invalid_state(
                "cleanup evidence does not belong to an active operation",
            ));
        }
        record.cleanup_errors.push(error);
        drop(registry);
        self.notify();
        Ok(())
    }

    pub(crate) fn abandon_operation(&self, operation: OperationId) -> Result<(), HostError> {
        let mut registry = self.lock()?;
        let record = registry.operation_mut(self.id, operation)?;
        record.owner_lost = true;
        if !record.dispatched {
            record.outcome = Some(ExternalOutcome::NotDispatched);
        }
        registry.settle(self.id);
        drop(registry);
        self.notify();
        Ok(())
    }

    pub(crate) fn record_progress(
        &self,
        operation: OperationId,
        completed_units: u64,
    ) -> Result<(), HostError> {
        let mut registry = self.lock()?;
        let record = registry.operation_mut(self.id, operation)?;
        if !record.dispatched
            || record.outcome.is_some()
            || record.owner_lost
            || record
                .completed_units
                .is_some_and(|previous| completed_units < previous)
        {
            return Err(invalid_state(
                "operation progress does not belong to an active dispatch",
            ));
        }
        record.completed_units = Some(completed_units);
        drop(registry);
        self.notify();
        Ok(())
    }

    pub(crate) fn request_stop(&self, reason: CancellationReason) -> Result<(), HostError> {
        let mut registry = self.lock()?;
        let cause = CancellationCause::new(self.id, reason);
        let mut pending = vec![self.id];
        let mut notifications = Vec::new();
        while let Some(id) = pending.pop() {
            let node = &mut registry.nodes[id.0 as usize];
            if node.state == ScopeState::Terminated {
                continue;
            }
            node.state = ScopeState::Stopping;
            if !node.causes.contains(&cause) {
                node.causes.push(cause.clone());
            }
            pending.extend(node.children.iter().copied());
            notifications.push(node.token.clone());
        }
        drop(registry);
        // Publish every descendant's cause before cancellation wakes any adapter.
        for token in notifications {
            token.cancel();
        }
        self.notify();
        Ok(())
    }

    /// The engine has finished its local body. Children and operations retain ownership.
    pub fn finish_body(&self, succeeded: bool) -> Result<(), HostError> {
        if !succeeded {
            self.request_stop(CancellationReason::ChildFailed)?;
        }
        let mut registry = self.lock()?;
        let node = &mut registry.nodes[self.id.0 as usize];
        if node.body_done {
            return Err(invalid_state("execution scope body completed twice"));
        }
        node.body_done = true;
        if node.state == ScopeState::Running {
            node.state = ScopeState::Draining;
        }
        registry.settle(self.id);
        drop(registry);
        self.notify();
        Ok(())
    }

    pub fn termination(&self) -> Result<Option<TerminationReport>, HostError> {
        Ok(self.lock()?.nodes[self.id.0 as usize].terminal.clone())
    }

    pub fn pending(&self) -> Result<PendingWork, HostError> {
        let registry = self.lock()?;
        let mut result = PendingWork {
            scopes: Vec::new(),
            operations: Vec::new(),
        };
        let mut pending = vec![self.id];
        while let Some(id) = pending.pop() {
            let node = &registry.nodes[id.0 as usize];
            if node.state != ScopeState::Terminated {
                result.scopes.push(id);
            }
            result.operations.extend(
                node.operations
                    .values()
                    .filter(|op| op.outcome.is_none())
                    .cloned(),
            );
            pending.extend(node.children.iter().copied());
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

impl Registry {
    fn check_admission(&self, id: ScopeId, nested: bool) -> Result<(), HostError> {
        let mut next = Some(id);
        while let Some(current) = next {
            let node = &self.nodes[current.0 as usize];
            match node.state {
                ScopeState::Stopping | ScopeState::Terminated => {
                    return Err(invalid_state("execution scope admission is closed"));
                }
                ScopeState::Draining if current == id && !nested => {
                    return Err(invalid_state("draining scope rejects unowned work"));
                }
                _ => {}
            }
            next = node.parent;
        }
        Ok(())
    }

    fn operation_mut(
        &mut self,
        scope: ScopeId,
        id: OperationId,
    ) -> Result<&mut OperationReport, HostError> {
        self.nodes[scope.0 as usize]
            .operations
            .get_mut(&id)
            .ok_or_else(|| invalid_state("operation is not registered in this scope"))
    }

    fn settle(&mut self, id: ScopeId) {
        // A dropped waiter delegates completion to its registered supervisors.
        // With no supervisor evidence, keep the lost owner pending instead.
        let operations = &mut self.nodes[id.0 as usize].operations;
        loop {
            let settled: Vec<_> = operations
                .values()
                .filter(|record| {
                    record.owner_lost
                        && record.outcome.is_none()
                        && operations
                            .values()
                            .any(|child| child.parent == Some(record.id))
                        && operations
                            .values()
                            .filter(|child| child.parent == Some(record.id))
                            .all(|child| child.outcome.is_some())
                })
                .map(|record| record.id)
                .collect();
            if settled.is_empty() {
                break;
            }
            for id in settled {
                if let Some(record) = operations.get_mut(&id) {
                    record.outcome = Some(match record.completed_units {
                        Some(completed_units) => ExternalOutcome::Partial { completed_units },
                        None => ExternalOutcome::Unknown,
                    });
                }
            }
        }
        let mut next = Some(id);
        while let Some(id) = next {
            let node = &self.nodes[id.0 as usize];
            if !node.body_done
                || node.terminal.is_some()
                || node.operations.values().any(|op| op.outcome.is_none())
                || node
                    .children
                    .iter()
                    .any(|child| self.nodes[child.0 as usize].terminal.is_none())
            {
                break;
            }
            let outcome = match node.causes.first() {
                Some(cause) if cause.reason() == &CancellationReason::ChildFailed => {
                    ScopeOutcome::Failed
                }
                Some(cause) => ScopeOutcome::Cancelled(cause.clone()),
                None => ScopeOutcome::Completed,
            };
            let mut report = TerminationReport {
                scope: id,
                outcome,
                causes: node.causes.clone(),
                operations: node.operations.values().cloned().collect(),
            };
            for child in &node.children {
                if let Some(child_report) = &self.nodes[child.0 as usize].terminal {
                    report
                        .operations
                        .extend(child_report.operations.iter().cloned());
                }
            }
            let node = &mut self.nodes[id.0 as usize];
            node.terminal = Some(report);
            node.state = ScopeState::Terminated;
            next = node.parent;
        }
    }
}
