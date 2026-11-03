//! A trace monitor bound to one checked obligation. It is not an installed
//! plan, evidence provider, or permission to dispatch.

use std::collections::BTreeMap;

use crate::{
    SourceRevisionKey,
    identity::canonical::CanonicalEncoder,
    residual::{
        binding::{BindingLayout, BindingLayoutError, BindingLayoutLimits},
        identity::{
            ActionPatternId, BindingLayoutId, MonitorTemplateId, ObligationIdentityError,
            ObligationIdentityLimits,
        },
        model::{ResidualKind, ResidualObligation},
        pattern::ActionPattern,
    },
};

use super::{
    AlternativeDecision, AlternativeDelta, AlternativeError, AlternativeLimits, AlternativeScope,
    AlternativeState, CompiledTraceAlternatives, ConcreteActionAttempt, ConcretePriorEvidence,
    EvidenceRequirements, ScopeCheckResult, ScopeExit, TraceAlternativesTemplate,
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
    monitor: CompiledTraceAlternatives,
    limits: TraceComponentLimits,
}

pub struct TraceComponentState {
    component: MonitorTemplateId,
    inner: AlternativeState,
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
            monitor,
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

    /// The normalized alternatives this component executes.
    pub fn template(&self) -> &TraceAlternativesTemplate {
        self.monitor.template()
    }

    /// The validated action patterns referenced by the template.
    pub fn patterns(&self) -> &BTreeMap<ActionPatternId, ActionPattern> {
        self.monitor.patterns()
    }

    pub fn limits(&self) -> TraceComponentLimits {
        self.limits
    }

    pub fn initial_state(
        &self,
        scope: AlternativeScope,
        source: SourceRevisionKey,
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
        Ok(TraceComponentState {
            component: self.id,
            inner: self.monitor.initial_state(scope),
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
        if scope.scope != self.obligation.identity().scope {
            return Err(TraceComponentError::ScopeMismatch);
        }
        if scope.activation != self.obligation.identity().activation {
            return Err(TraceComponentError::ActivationMismatch);
        }
        if source != self.obligation.identity().origin.revision {
            return Err(TraceComponentError::SourceRevisionMismatch);
        }
        Ok(TraceComponentState {
            component: self.id,
            inner: self
                .monitor
                .restore_state(scope, data)
                .map_err(TraceComponentError::Monitor)?,
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
            match self
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
        self.monitor
            .finish_scope_refs(
                &state.inner,
                pending.iter().map(|request| &request.inner),
                exit,
            )
            .map_err(TraceComponentError::Monitor)
    }
}
