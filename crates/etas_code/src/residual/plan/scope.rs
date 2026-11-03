//! Pure scope composition. This is not native site installation, complete
//! dispatch-site coverage, or authorization to execute an action.

use std::collections::BTreeMap;

use crate::{
    contract::CanonicalEffect,
    residual::{
        identity::{
            ActivationSiteId, GenericArgumentIdentity, GenericParameterId, MonitorTemplateId,
            PlanHash, SemanticScopeId,
        },
        model::{ActionPredicateDecision, ActionPredicateError},
        monitor::{
            ActionAttemptId, ActionComponentInput, AlternativeScope, BoundTraceComponent,
            ConcreteActionAttempt, ConcretePriorEvidence, ScopeCheckResult, ScopeExit,
            ScopeInvocationId, SemanticEventCursor, TraceBatchCommitError, TraceComponentDecision,
            TraceComponentDelta, TraceComponentError, TraceComponentEvidenceRequirements,
            TraceComponentState, TraceComponentTransition, commit_trace_transitions,
        },
        predicate::PredicateDecision,
        type_binding::TypeArgumentDescriptors,
    },
};

use super::{SemanticPlanComponent, SemanticResidualPlan};

pub struct ResidualScopeEvent<'a> {
    pub id: ActionAttemptId,
    pub cursor: SemanticEventCursor,
    pub action: &'a CanonicalEffect,
}

pub enum ResidualComponentAttempt<'a> {
    Action(ResidualActionAttempt<'a>),
    Trace(ConcreteActionAttempt<'a>),
}

/// Checked native data for the same event being evaluated by every component.
/// The adapter remains responsible for proving the data's source and ownership.
pub struct ResidualActionAttempt<'a> {
    pub id: ActionAttemptId,
    pub cursor: SemanticEventCursor,
    pub revision: crate::RevisionKey,
    pub invocation: ScopeInvocationId,
    pub input: ActionComponentInput<'a>,
}

enum ComponentState {
    Action,
    Trace(Box<TraceComponentState>),
}

pub struct ResidualScopeState {
    plan: PlanHash,
    scope: AlternativeScope,
    version: u64,
    last_cursor: Option<SemanticEventCursor>,
    components: BTreeMap<MonitorTemplateId, ComponentState>,
}

/// Immutable native factory for one checked type application. Invocation state
/// is allocated separately; generic monitor compilation is shared by calls.
#[derive(Clone, Debug)]
pub struct SpecializedResidualScope {
    plan: PlanHash,
    scope: SemanticScopeId,
    activation: ActivationSiteId,
    traces: BTreeMap<MonitorTemplateId, BoundTraceComponent>,
}

/// Data-only checkpoint form of one scope's monitor state. It names its plan
/// and components but is untrusted until restored through that plan.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidualScopeStateData {
    pub plan: PlanHash,
    pub version: u64,
    pub last_cursor: Option<u64>,
    pub components: Vec<ResidualComponentStateData>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidualComponentStateData {
    pub component: MonitorTemplateId,
    pub trace: Option<crate::residual::monitor::AlternativeStateData>,
}

impl ResidualScopeState {
    pub fn scope(&self) -> AlternativeScope {
        self.scope
    }

    pub fn to_data(&self) -> ResidualScopeStateData {
        ResidualScopeStateData {
            plan: self.plan,
            version: self.version,
            last_cursor: self.last_cursor.map(|cursor| cursor.0),
            components: self
                .components
                .iter()
                .map(|(component, state)| ResidualComponentStateData {
                    component: *component,
                    trace: match state {
                        ComponentState::Action => None,
                        ComponentState::Trace(state) => Some(state.to_data()),
                    },
                })
                .collect(),
        }
    }
    pub fn component_ids(&self) -> impl Iterator<Item = MonitorTemplateId> + '_ {
        self.components.keys().copied()
    }
}

pub struct ResidualScopeDelta {
    plan: PlanHash,
    scope: AlternativeScope,
    version: u64,
    cursor: SemanticEventCursor,
    traces: BTreeMap<MonitorTemplateId, TraceComponentDelta>,
}

pub struct ResidualScopePending {
    pub component: MonitorTemplateId,
    pub requirements: TraceComponentEvidenceRequirements,
}

pub enum ResidualScopeDecision {
    Permit(ResidualScopeDelta),
    NeedsEvidence(Vec<ResidualScopePending>),
    Reject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidualScopeError {
    NoComponents,
    ComponentLimit,
    PendingLimit,
    PlanMismatch,
    ScopeMismatch,
    ComponentCoverage,
    EventMismatch(MonitorTemplateId),
    UnresolvedSelector(MonitorTemplateId),
    MissingActionBinding(MonitorTemplateId),
    UnsupportedPredicate(MonitorTemplateId),
    StaleEvent,
    StaleTransition,
    VersionExhausted,
    InvalidPending,
    Action {
        id: MonitorTemplateId,
        error: ActionPredicateError,
    },
    Component {
        id: MonitorTemplateId,
        error: TraceComponentError,
    },
    Commit(TraceBatchCommitError),
}

impl SemanticResidualPlan {
    pub fn specialize_scope(
        &self,
        scope: SemanticScopeId,
        activation: ActivationSiteId,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        types: &TypeArgumentDescriptors,
        max_components: usize,
    ) -> Result<SpecializedResidualScope, ResidualScopeError> {
        let mut traces = BTreeMap::new();
        let mut count = 0;
        for (id, component) in &self.components {
            let identity = component.obligation().identity();
            if identity.scope != scope || identity.activation != activation {
                continue;
            }
            count += 1;
            if count > max_components {
                return Err(ResidualScopeError::ComponentLimit);
            }
            if let SemanticPlanComponent::Trace(trace) = component {
                traces.insert(
                    *id,
                    trace
                        .bind_types(bindings, types)
                        .map_err(|error| ResidualScopeError::Component { id: *id, error })?,
                );
            }
        }
        if count == 0 {
            return Err(ResidualScopeError::NoComponents);
        }
        Ok(SpecializedResidualScope {
            plan: self.hash,
            scope,
            activation,
            traces,
        })
    }
    /// Activates every component for the scope. The native engine must still
    /// validate source/owner bindings and ensure unique invocation identities.
    pub fn initial_scope(
        &self,
        scope: AlternativeScope,
        max_components: usize,
    ) -> Result<ResidualScopeState, ResidualScopeError> {
        self.initial_scope_with_bindings(
            scope,
            max_components,
            &BTreeMap::new(),
            &TypeArgumentDescriptors::default(),
        )
    }

    pub fn initial_scope_with_bindings(
        &self,
        scope: AlternativeScope,
        max_components: usize,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        type_descriptors: &TypeArgumentDescriptors,
    ) -> Result<ResidualScopeState, ResidualScopeError> {
        let prepared = self.specialize_scope(
            scope.scope,
            scope.activation,
            bindings,
            type_descriptors,
            max_components,
        )?;
        self.initial_specialized_scope(scope, max_components, &prepared)
    }

    pub fn initial_specialized_scope(
        &self,
        scope: AlternativeScope,
        max_components: usize,
        prepared: &SpecializedResidualScope,
    ) -> Result<ResidualScopeState, ResidualScopeError> {
        self.validate_specialized_scope(scope, prepared)?;
        let mut components = BTreeMap::new();
        for (id, component) in &self.components {
            let obligation = component.obligation();
            if obligation.identity().scope != scope.scope
                || obligation.identity().activation != scope.activation
            {
                continue;
            }
            if components.len() >= max_components {
                return Err(ResidualScopeError::ComponentLimit);
            }
            let state = match component {
                SemanticPlanComponent::Action(_) => ComponentState::Action,
                SemanticPlanComponent::Trace(trace) => ComponentState::Trace(Box::new(
                    trace
                        .initial_bound_state(
                            scope,
                            obligation.identity().origin.revision,
                            prepared
                                .traces
                                .get(id)
                                .ok_or(ResidualScopeError::ComponentCoverage)?,
                        )
                        .map_err(|error| ResidualScopeError::Component { id: *id, error })?,
                )),
            };
            components.insert(*id, state);
        }
        if components.is_empty() {
            return Err(ResidualScopeError::NoComponents);
        }
        Ok(ResidualScopeState {
            plan: self.hash,
            scope,
            version: 0,
            last_cursor: None,
            components,
        })
    }

    /// Restores a scope from untrusted checkpoint data. The data must name
    /// this plan and exactly the components `initial_scope` activates for the
    /// scope, and each component state must be valid for its monitor.
    pub fn restore_scope(
        &self,
        scope: AlternativeScope,
        data: &ResidualScopeStateData,
        max_components: usize,
    ) -> Result<ResidualScopeState, ResidualScopeError> {
        self.restore_scope_with_bindings(
            scope,
            data,
            max_components,
            &BTreeMap::new(),
            &TypeArgumentDescriptors::default(),
        )
    }

    pub fn restore_scope_with_bindings(
        &self,
        scope: AlternativeScope,
        data: &ResidualScopeStateData,
        max_components: usize,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        type_descriptors: &TypeArgumentDescriptors,
    ) -> Result<ResidualScopeState, ResidualScopeError> {
        let prepared = self.specialize_scope(
            scope.scope,
            scope.activation,
            bindings,
            type_descriptors,
            max_components,
        )?;
        self.restore_specialized_scope(scope, data, max_components, &prepared)
    }

    fn validate_specialized_scope(
        &self,
        scope: AlternativeScope,
        prepared: &SpecializedResidualScope,
    ) -> Result<(), ResidualScopeError> {
        if prepared.plan != self.hash {
            return Err(ResidualScopeError::PlanMismatch);
        }
        if prepared.scope != scope.scope || prepared.activation != scope.activation {
            return Err(ResidualScopeError::ScopeMismatch);
        }
        Ok(())
    }

    pub fn restore_specialized_scope(
        &self,
        scope: AlternativeScope,
        data: &ResidualScopeStateData,
        max_components: usize,
        prepared: &SpecializedResidualScope,
    ) -> Result<ResidualScopeState, ResidualScopeError> {
        self.validate_specialized_scope(scope, prepared)?;
        if data.plan != self.hash {
            return Err(ResidualScopeError::PlanMismatch);
        }
        if data.components.len() > max_components {
            return Err(ResidualScopeError::ComponentLimit);
        }
        let mut expected = BTreeMap::new();
        for (id, component) in &self.components {
            let identity = component.obligation().identity();
            if identity.scope == scope.scope && identity.activation == scope.activation {
                expected.insert(*id, component);
            }
        }
        if expected.is_empty() || expected.len() != data.components.len() {
            return Err(ResidualScopeError::ComponentCoverage);
        }
        let mut components = BTreeMap::new();
        for entry in &data.components {
            let component = expected
                .get(&entry.component)
                .ok_or(ResidualScopeError::ComponentCoverage)?;
            let state = match (component, &entry.trace) {
                (SemanticPlanComponent::Action(_), None) => ComponentState::Action,
                (SemanticPlanComponent::Trace(trace), Some(state)) => {
                    ComponentState::Trace(Box::new(
                        trace
                            .restore_bound_state(
                                scope,
                                component.obligation().identity().origin.revision,
                                state,
                                prepared
                                    .traces
                                    .get(&entry.component)
                                    .ok_or(ResidualScopeError::ComponentCoverage)?,
                            )
                            .map_err(|error| ResidualScopeError::Component {
                                id: entry.component,
                                error,
                            })?,
                    ))
                }
                _ => return Err(ResidualScopeError::ComponentCoverage),
            };
            if components.insert(entry.component, state).is_some() {
                return Err(ResidualScopeError::ComponentCoverage);
            }
        }
        Ok(ResidualScopeState {
            plan: self.hash,
            scope,
            version: data.version,
            last_cursor: data.last_cursor.map(SemanticEventCursor),
            components,
        })
    }

    /// Every active component must receive an adapter view of the same event.
    /// Evidence requests are not permits and require a fresh preview.
    pub fn preview_scope<E: ConcretePriorEvidence>(
        &self,
        state: &ResidualScopeState,
        event: ResidualScopeEvent<'_>,
        attempts: &BTreeMap<MonitorTemplateId, ResidualComponentAttempt<'_>>,
        evidence: &E,
    ) -> Result<ResidualScopeDecision, ResidualScopeError> {
        if state.plan != self.hash {
            return Err(ResidualScopeError::PlanMismatch);
        }
        if state.last_cursor.is_some_and(|last| event.cursor <= last) {
            return Err(ResidualScopeError::StaleEvent);
        }
        if attempts.len() != state.components.len() {
            return Err(ResidualScopeError::ComponentCoverage);
        }
        let mut traces = BTreeMap::new();
        let mut pending = Vec::new();
        let mut rejected = false;
        for (id, component_state) in &state.components {
            let attempt = attempts
                .get(id)
                .ok_or(ResidualScopeError::ComponentCoverage)?;
            let component = self
                .components
                .get(id)
                .ok_or(ResidualScopeError::ComponentCoverage)?;
            match (component, component_state, attempt) {
                (
                    SemanticPlanComponent::Action(component),
                    ComponentState::Action,
                    ResidualComponentAttempt::Action(attempt),
                ) => {
                    if attempt.input.scope != state.scope.scope
                        || attempt.input.activation != state.scope.activation
                        || attempt.revision != state.scope.revision
                        || attempt.invocation != state.scope.invocation
                        || attempt.id != event.id
                        || attempt.cursor != event.cursor
                        || attempt.input.action != event.action
                    {
                        return Err(ResidualScopeError::EventMismatch(*id));
                    }
                    match component
                        .evaluate(attempt.input)
                        .map_err(|error| ResidualScopeError::Action { id: *id, error })?
                    {
                        ActionPredicateDecision::NotApplicable
                        | ActionPredicateDecision::Applicable(PredicateDecision::Satisfied) => {}
                        ActionPredicateDecision::Applicable(PredicateDecision::Violated) => {
                            rejected = true
                        }
                        ActionPredicateDecision::Applicable(PredicateDecision::NeedsEvidence) => {
                            return Err(ResidualScopeError::MissingActionBinding(*id));
                        }
                        ActionPredicateDecision::Applicable(PredicateDecision::Unsupported) => {
                            return Err(ResidualScopeError::UnsupportedPredicate(*id));
                        }
                        ActionPredicateDecision::NeedsConcreteSelector => {
                            return Err(ResidualScopeError::UnresolvedSelector(*id));
                        }
                    }
                }
                (
                    SemanticPlanComponent::Trace(component),
                    ComponentState::Trace(component_state),
                    ResidualComponentAttempt::Trace(attempt),
                ) => {
                    if attempt.scope != state.scope.scope
                        || attempt.activation != state.scope.activation
                        || attempt.revision != state.scope.revision
                        || attempt.invocation != state.scope.invocation
                        || attempt.id != event.id
                        || attempt.cursor != event.cursor
                        || attempt.action != event.action
                    {
                        return Err(ResidualScopeError::EventMismatch(*id));
                    }
                    match component
                        .preview(component_state, attempt, evidence)
                        .map_err(|error| ResidualScopeError::Component { id: *id, error })?
                    {
                        TraceComponentDecision::Permit(delta) => {
                            traces.insert(*id, delta);
                        }
                        TraceComponentDecision::NeedsEvidence(requirements) => {
                            pending.push(ResidualScopePending {
                                component: *id,
                                requirements,
                            })
                        }
                        TraceComponentDecision::Reject => rejected = true,
                    }
                }
                _ => return Err(ResidualScopeError::ComponentCoverage),
            }
        }
        if rejected {
            return Ok(ResidualScopeDecision::Reject);
        }
        if !pending.is_empty() {
            return Ok(ResidualScopeDecision::NeedsEvidence(pending));
        }
        Ok(ResidualScopeDecision::Permit(ResidualScopeDelta {
            plan: self.hash,
            scope: state.scope,
            version: state.version,
            cursor: event.cursor,
            traces,
        }))
    }

    /// Validates the scope and all trace transitions before mutating any state.
    pub fn commit_scope(
        &self,
        state: &mut ResidualScopeState,
        delta: ResidualScopeDelta,
    ) -> Result<(), ResidualScopeError> {
        if state.plan != self.hash || delta.plan != self.hash {
            return Err(ResidualScopeError::PlanMismatch);
        }
        if state.scope != delta.scope {
            return Err(ResidualScopeError::ScopeMismatch);
        }
        if state.version != delta.version {
            return Err(ResidualScopeError::StaleTransition);
        }
        if state.last_cursor.is_some_and(|last| delta.cursor <= last) {
            return Err(ResidualScopeError::StaleEvent);
        }
        let version = state
            .version
            .checked_add(1)
            .ok_or(ResidualScopeError::VersionExhausted)?;
        let expected = state
            .components
            .values()
            .filter(|value| matches!(value, ComponentState::Trace(_)))
            .count();
        if expected != delta.traces.len() {
            return Err(ResidualScopeError::ComponentCoverage);
        }
        let mut deltas = delta.traces.into_iter();
        let mut transitions = Vec::with_capacity(expected);
        for (id, component_state) in &mut state.components {
            let ComponentState::Trace(component_state) = component_state else {
                continue;
            };
            let (delta_id, component_delta) =
                deltas.next().ok_or(ResidualScopeError::ComponentCoverage)?;
            if *id != delta_id {
                return Err(ResidualScopeError::ComponentCoverage);
            }
            let Some(SemanticPlanComponent::Trace(component)) = self.components.get(id) else {
                return Err(ResidualScopeError::ComponentCoverage);
            };
            transitions.push(TraceComponentTransition {
                component,
                state: component_state,
                delta: component_delta,
            });
        }
        if deltas.next().is_some() {
            return Err(ResidualScopeError::ComponentCoverage);
        }
        commit_trace_transitions(transitions, expected).map_err(ResidualScopeError::Commit)?;
        state.version = version;
        state.last_cursor = Some(delta.cursor);
        Ok(())
    }

    /// The native engine must supply every pending request; this method cannot
    /// prove retention or replace runtime installation evidence.
    pub fn finish_scope(
        &self,
        state: &ResidualScopeState,
        pending: &[ResidualScopePending],
        exit: ScopeExit,
        max_pending: usize,
    ) -> Result<ScopeCheckResult, ResidualScopeError> {
        if state.plan != self.hash {
            return Err(ResidualScopeError::PlanMismatch);
        }
        if pending.len() > max_pending {
            return Err(ResidualScopeError::PendingLimit);
        }
        let mut trace_pending: BTreeMap<
            MonitorTemplateId,
            Vec<TraceComponentEvidenceRequirements>,
        > = BTreeMap::new();
        for request in pending {
            if !matches!(
                state.components.get(&request.component),
                Some(ComponentState::Trace(_))
            ) {
                return Err(ResidualScopeError::InvalidPending);
            }
            trace_pending
                .entry(request.component)
                .or_default()
                .push(request.requirements.clone());
        }
        let mut rejected = false;
        let mut incomplete = false;
        let mut waiting = false;
        for (id, component_state) in &state.components {
            let ComponentState::Trace(component_state) = component_state else {
                continue;
            };
            let Some(SemanticPlanComponent::Trace(component)) = self.components.get(id) else {
                return Err(ResidualScopeError::ComponentCoverage);
            };
            match component
                .finish_scope(
                    component_state,
                    trace_pending.get(id).map_or(&[], Vec::as_slice),
                    exit,
                )
                .map_err(|error| ResidualScopeError::Component { id: *id, error })?
            {
                ScopeCheckResult::Accepting => {}
                ScopeCheckResult::Rejected => rejected = true,
                ScopeCheckResult::PendingEvidence => waiting = true,
                ScopeCheckResult::Incomplete => incomplete = true,
            }
        }
        Ok(if rejected {
            ScopeCheckResult::Rejected
        } else if exit != ScopeExit::Completed || incomplete {
            ScopeCheckResult::Incomplete
        } else if waiting {
            ScopeCheckResult::PendingEvidence
        } else {
            ScopeCheckResult::Accepting
        })
    }
}
