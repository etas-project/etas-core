//! A trace monitor bound to one checked obligation. It is not an installed
//! plan, evidence provider, or permission to dispatch.

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    SourceRevisionKey,
    identity::canonical::CanonicalEncoder,
    residual::{
        binding::{BindingLayout, BindingLayoutError, BindingLayoutLimits},
        identity::{
            ActionPatternId, BindingLayoutId, GenericArgumentIdentity, GenericParameterId,
            MonitorTemplateId, ObligationIdentityError, ObligationIdentityLimits,
        },
        model::{ResidualKind, ResidualObligation},
        pattern::{ActionPattern, PatternTemplateLimits},
        type_template::StaticTypeTemplate,
    },
};

use super::{
    AlternativeDecision, AlternativeDelta, AlternativeError, AlternativeLimits, AlternativeScope,
    AlternativeState, CompiledTraceAlternatives, ConcreteActionAttempt, ConcretePriorEvidence,
    EvidenceRequirements, ScopeCheckResult, ScopeExit, SymbolicTraceAlternatives,
    SymbolicTraceError, TraceAlternativesTemplate,
};

#[derive(Clone, Copy, Debug)]
pub struct TraceComponentLimits {
    pub identity: ObligationIdentityLimits,
    pub layout: BindingLayoutLimits,
    pub monitor: AlternativeLimits,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceComponentError {
    Identity(ObligationIdentityError),
    Layout(BindingLayoutError),
    Monitor(AlternativeError),
    Symbolic(SymbolicTraceError),
    NotTraceContract,
    TemplateMismatch,
    LayoutMismatch,
    BindingLimit,
    MissingBindings { count: usize },
    ScopeMismatch,
    ActivationMismatch,
    SourceRevisionMismatch,
    ComponentMismatch,
    IdentityMismatch,
    LimitEncoding,
}

pub struct CompiledTraceComponent {
    id: MonitorTemplateId,
    obligation: ResidualObligation,
    layout: BindingLayoutId,
    monitor: TraceComponentMonitor,
    limits: TraceComponentLimits,
}

pub struct TraceComponentState {
    component: MonitorTemplateId,
    monitor: Arc<CompiledTraceAlternatives>,
    inner: AlternativeState,
}

pub enum TraceComponentMonitor {
    Concrete(Arc<CompiledTraceAlternatives>),
    Symbolic(SymbolicTraceAlternatives),
}

impl TraceComponentMonitor {
    pub fn template_id(&self) -> MonitorTemplateId {
        match self {
            Self::Concrete(monitor) => monitor.template_id(),
            Self::Symbolic(monitor) => {
                MonitorTemplateId::from_canonical_fingerprint(monitor.fingerprint())
            }
        }
    }

    pub fn compile_component(
        &self,
        obligation: ResidualObligation,
        layout: &BindingLayout,
        limits: TraceComponentLimits,
    ) -> Result<CompiledTraceComponent, TraceComponentError> {
        match self {
            Self::Concrete(monitor) => CompiledTraceComponent::compile(
                obligation,
                monitor.template(),
                monitor.patterns(),
                layout,
                limits,
            ),
            Self::Symbolic(monitor) => CompiledTraceComponent::compile_symbolic(
                obligation,
                monitor.clone(),
                layout,
                limits,
            ),
        }
    }
}

pub struct TraceComponentDelta {
    component: MonitorTemplateId,
    inner: AlternativeDelta,
}

/// A transition supplied by a native adapter for an already selected scope.
/// This association does not establish plan coverage or dispatch authority.
pub struct TraceComponentTransition<'a> {
    pub component: &'a CompiledTraceComponent,
    pub state: &'a mut TraceComponentState,
    pub delta: TraceComponentDelta,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceBatchCommitError {
    TransitionLimit,
    InvalidTransition {
        index: usize,
        error: TraceComponentError,
    },
}

/// Commits a bounded group only after every transition has been validated.
/// The caller must select the required scopes and obtain all decisions first.
pub fn commit_trace_transitions(
    transitions: Vec<TraceComponentTransition<'_>>,
    max_transitions: usize,
) -> Result<(), TraceBatchCommitError> {
    if transitions.len() > max_transitions {
        return Err(TraceBatchCommitError::TransitionLimit);
    }
    let mut versions = Vec::with_capacity(transitions.len());
    for (index, transition) in transitions.iter().enumerate() {
        let version = transition
            .component
            .validate_transition(transition.state, &transition.delta)
            .map_err(|error| TraceBatchCommitError::InvalidTransition { index, error })?;
        versions.push(version);
    }
    for (transition, version) in transitions.into_iter().zip(versions) {
        transition
            .state
            .inner
            .apply_validated(transition.delta.inner, version);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceComponentEvidenceRequirements {
    component: MonitorTemplateId,
    inner: EvidenceRequirements,
}

impl TraceComponentState {
    pub fn component(&self) -> MonitorTemplateId {
        self.component
    }
    pub fn to_data(&self) -> super::AlternativeStateData {
        self.inner.to_data()
    }
}

impl TraceComponentEvidenceRequirements {
    pub fn requirements(&self) -> &EvidenceRequirements {
        &self.inner
    }
}

pub enum TraceComponentDecision {
    Permit(TraceComponentDelta),
    NeedsEvidence(TraceComponentEvidenceRequirements),
    Reject,
}

impl CompiledTraceComponent {
    pub fn compile(
        obligation: ResidualObligation,
        template: &TraceAlternativesTemplate,
        patterns: &BTreeMap<ActionPatternId, ActionPattern>,
        layout: &BindingLayout,
        limits: TraceComponentLimits,
    ) -> Result<Self, TraceComponentError> {
        obligation
            .id()
            .validate(obligation.identity(), limits.identity)
            .map_err(TraceComponentError::Identity)?;
        layout
            .validate_with_limits(limits.layout)
            .map_err(TraceComponentError::Layout)?;
        if layout.id() != obligation.bindings() {
            return Err(TraceComponentError::LayoutMismatch);
        }
        if layout.len() > limits.monitor.evidence.max_bindings {
            return Err(TraceComponentError::BindingLimit);
        }
        let ResidualKind::TraceContract { monitor: expected } = obligation.kind() else {
            return Err(TraceComponentError::NotTraceContract);
        };
        let monitor = CompiledTraceAlternatives::from_template(template, patterns, limits.monitor)
            .map_err(TraceComponentError::Monitor)?;
        if monitor.template_id() != *expected {
            return Err(TraceComponentError::TemplateMismatch);
        }
        let mut encoder = CanonicalEncoder::new("etas.code.residual.trace-component.v1");
        encoder.fingerprint(obligation.id().fingerprint());
        encoder.fingerprint(monitor.runtime_id());
        encoder.fingerprint(layout.id().fingerprint());
        for limit in [
            limits.identity.max_generic_arguments,
            limits.identity.max_structural_site_depth,
            limits.layout.max_bindings,
            limits.layout.max_projections_per_binding,
            limits.layout.max_total_projections,
        ] {
            encoder.number(u64::try_from(limit).map_err(|_| TraceComponentError::LimitEncoding)?);
        }
        Ok(Self {
            id: MonitorTemplateId::from_canonical_fingerprint(encoder.finish()),
            obligation,
            layout: layout.id(),
            monitor: TraceComponentMonitor::Concrete(Arc::new(monitor)),
            limits,
        })
    }

    pub fn compile_symbolic(
        obligation: ResidualObligation,
        monitor: SymbolicTraceAlternatives,
        layout: &BindingLayout,
        limits: TraceComponentLimits,
    ) -> Result<Self, TraceComponentError> {
        obligation
            .id()
            .validate(obligation.identity(), limits.identity)
            .map_err(TraceComponentError::Identity)?;
        layout
            .validate_with_limits(limits.layout)
            .map_err(TraceComponentError::Layout)?;
        if layout.id() != obligation.bindings() {
            return Err(TraceComponentError::LayoutMismatch);
        }
        if layout.len() > limits.monitor.evidence.max_bindings {
            return Err(TraceComponentError::BindingLimit);
        }
        let ResidualKind::TraceContract { monitor: expected } = obligation.kind() else {
            return Err(TraceComponentError::NotTraceContract);
        };
        if expected.fingerprint() != monitor.fingerprint() {
            return Err(TraceComponentError::TemplateMismatch);
        }
        let mut encoder = CanonicalEncoder::new("etas.code.residual.symbolic-trace-component.v1");
        encoder.fingerprint(obligation.id().fingerprint());
        encoder.fingerprint(monitor.fingerprint());
        encoder.fingerprint(layout.id().fingerprint());
        for limit in [
            limits.identity.max_generic_arguments,
            limits.identity.max_structural_site_depth,
            limits.layout.max_bindings,
            limits.layout.max_projections_per_binding,
            limits.layout.max_total_projections,
            limits.monitor.max_alternatives,
            limits.monitor.max_clauses,
            limits.monitor.max_patterns,
            limits.monitor.max_checks_per_event,
            limits.monitor.max_product_work_per_event,
            limits.monitor.evidence.max_bindings,
            limits.monitor.evidence.max_value_bytes,
            limits.monitor.evidence.pattern.max_selectors,
            limits.monitor.evidence.pattern.max_identity_segments,
            limits.monitor.evidence.pattern.max_identity_bytes,
            limits.monitor.evidence.pattern.max_literal_bytes,
        ] {
            encoder.number(u64::try_from(limit).map_err(|_| TraceComponentError::LimitEncoding)?);
        }
        Ok(Self {
            id: MonitorTemplateId::from_canonical_fingerprint(encoder.finish()),
            obligation,
            layout: layout.id(),
            monitor: TraceComponentMonitor::Symbolic(monitor),
            limits,
        })
    }

    pub fn id(&self) -> MonitorTemplateId {
        self.id
    }

    pub fn from_parts(
        id: MonitorTemplateId,
        obligation: ResidualObligation,
        template: &TraceAlternativesTemplate,
        patterns: &BTreeMap<ActionPatternId, ActionPattern>,
        layout: &BindingLayout,
        limits: TraceComponentLimits,
    ) -> Result<Self, TraceComponentError> {
        let component = Self::compile(obligation, template, patterns, layout, limits)?;
        if component.id != id {
            return Err(TraceComponentError::IdentityMismatch);
        }
        Ok(component)
    }
    pub fn obligation(&self) -> &ResidualObligation {
        &self.obligation
    }

    pub fn monitor(&self) -> &TraceComponentMonitor {
        &self.monitor
    }

    pub fn limits(&self) -> TraceComponentLimits {
        self.limits
    }

    pub fn initial_state(
        &self,
        scope: AlternativeScope,
        source: SourceRevisionKey,
    ) -> Result<TraceComponentState, TraceComponentError> {
        self.initial_state_with_bindings(scope, source, &BTreeMap::new(), &BTreeMap::new())
    }

    fn bind_monitor(
        &self,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        structures: &BTreeMap<GenericParameterId, StaticTypeTemplate>,
    ) -> Result<Arc<CompiledTraceAlternatives>, TraceComponentError> {
        match &self.monitor {
            TraceComponentMonitor::Concrete(monitor) => Ok(monitor.clone()),
            TraceComponentMonitor::Symbolic(monitor) => monitor
                .instantiate(
                    bindings,
                    structures,
                    PatternTemplateLimits {
                        pattern: self.limits.monitor.evidence.pattern,
                        types: Default::default(),
                        descriptor: Default::default(),
                        max_binding_steps:
                            super::super::type_template::TypeTemplateLimits::default().max_work,
                    },
                    self.limits.monitor,
                )
                .map(Arc::new)
                .map_err(TraceComponentError::Symbolic),
        }
    }

    pub fn initial_state_with_bindings(
        &self,
        scope: AlternativeScope,
        source: SourceRevisionKey,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        structures: &BTreeMap<GenericParameterId, StaticTypeTemplate>,
    ) -> Result<TraceComponentState, TraceComponentError> {
        if scope.scope != self.obligation.identity().scope {
            return Err(TraceComponentError::ScopeMismatch);
        }
        if scope.activation != self.obligation.identity().activation {
            return Err(TraceComponentError::ActivationMismatch);
        }
        if source != self.obligation.identity().origin.revision {
            return Err(TraceComponentError::SourceRevisionMismatch);
        }
        let monitor = self.bind_monitor(bindings, structures)?;
        Ok(TraceComponentState {
            component: self.id,
            inner: monitor.initial_state(scope),
            monitor,
        })
    }

    /// Restores this component's state from untrusted checkpoint data for
    /// the same scope binding `initial_state` would accept.
    pub fn restore_state(
        &self,
        scope: AlternativeScope,
        source: SourceRevisionKey,
        data: &super::AlternativeStateData,
    ) -> Result<TraceComponentState, TraceComponentError> {
        self.restore_state_with_bindings(scope, source, data, &BTreeMap::new(), &BTreeMap::new())
    }

    pub fn restore_state_with_bindings(
        &self,
        scope: AlternativeScope,
        source: SourceRevisionKey,
        data: &super::AlternativeStateData,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        structures: &BTreeMap<GenericParameterId, StaticTypeTemplate>,
    ) -> Result<TraceComponentState, TraceComponentError> {
        if scope.scope != self.obligation.identity().scope {
            return Err(TraceComponentError::ScopeMismatch);
        }
        if scope.activation != self.obligation.identity().activation {
            return Err(TraceComponentError::ActivationMismatch);
        }
        if source != self.obligation.identity().origin.revision {
            return Err(TraceComponentError::SourceRevisionMismatch);
        }
        let monitor = self.bind_monitor(bindings, structures)?;
        Ok(TraceComponentState {
            component: self.id,
            inner: monitor
                .restore_state(scope, data)
                .map_err(TraceComponentError::Monitor)?,
            monitor,
        })
    }

    pub fn preview<E: ConcretePriorEvidence>(
        &self,
        state: &TraceComponentState,
        attempt: &ConcreteActionAttempt<'_>,
        evidence: &E,
    ) -> Result<TraceComponentDecision, TraceComponentError> {
        if state.component != self.id {
            return Err(TraceComponentError::ComponentMismatch);
        }
        if attempt.bindings.layout_id() != self.layout {
            return Err(TraceComponentError::LayoutMismatch);
        }
        if attempt.bindings.missing_count() != 0 {
            return Err(TraceComponentError::MissingBindings {
                count: attempt.bindings.missing_count(),
            });
        }
        Ok(
            match state
                .monitor
                .preview(&state.inner, attempt, evidence)
                .map_err(TraceComponentError::Monitor)?
            {
                AlternativeDecision::Permit(inner) => {
                    TraceComponentDecision::Permit(TraceComponentDelta {
                        component: self.id,
                        inner,
                    })
                }
                AlternativeDecision::NeedsEvidence(inner) => {
                    TraceComponentDecision::NeedsEvidence(TraceComponentEvidenceRequirements {
                        component: self.id,
                        inner,
                    })
                }
                AlternativeDecision::Reject => TraceComponentDecision::Reject,
            },
        )
    }

    pub fn commit(
        &self,
        state: &mut TraceComponentState,
        delta: TraceComponentDelta,
    ) -> Result<(), TraceComponentError> {
        let version = self.validate_transition(state, &delta)?;
        state.inner.apply_validated(delta.inner, version);
        Ok(())
    }

    fn validate_transition(
        &self,
        state: &TraceComponentState,
        delta: &TraceComponentDelta,
    ) -> Result<u64, TraceComponentError> {
        if state.component != self.id || delta.component != self.id {
            return Err(TraceComponentError::ComponentMismatch);
        }
        state
            .inner
            .validate_transition(&delta.inner)
            .map_err(TraceComponentError::Monitor)
    }

    /// The engine must supply its complete pending set; this check cannot
    /// establish that a caller has retained every outstanding request.
    pub fn finish_scope(
        &self,
        state: &TraceComponentState,
        pending: &[TraceComponentEvidenceRequirements],
        exit: ScopeExit,
    ) -> Result<ScopeCheckResult, TraceComponentError> {
        if state.component != self.id {
            return Err(TraceComponentError::ComponentMismatch);
        }
        if pending.len() > self.limits.monitor.max_checks_per_event {
            return Err(TraceComponentError::Monitor(AlternativeError::CheckLimit));
        }
        for request in pending {
            if request.component != self.id {
                return Err(TraceComponentError::ComponentMismatch);
            }
        }
        state
            .monitor
            .finish_scope_refs(
                &state.inner,
                pending.iter().map(|request| &request.inner),
                exit,
            )
            .map_err(TraceComponentError::Monitor)
    }
}
