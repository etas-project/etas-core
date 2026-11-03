//! Reference validation only. A successful result is not a verified plan or
//! evidence of backend support or runtime installation.

use std::collections::BTreeMap;

mod action_coverage;
mod trace_coverage;
pub use action_coverage::{
    ActionComponentCoverage, ActionCoverageEntry, ActionCoverageError, ActionCoverageLimits,
    verify_action_component_coverage,
};
pub use trace_coverage::{
    TraceComponentCoverage, TraceCoverageEntry, TraceCoverageError, TraceCoverageLimits,
    verify_trace_component_coverage,
};

use super::{
    binding::BindingLayout,
    identity::{ActionPatternId, BindingLayoutId, MonitorTemplateId, ObligationId, PredicateId},
    model::{ResidualBindingError, ResidualKind, ResidualObligationSet},
    monitor::CompiledTraceAlternatives,
    pattern::ActionPattern,
    predicate::BoundPredicateGraph,
};

#[derive(Default)]
pub struct ResidualReferenceCatalog<'a> {
    pub patterns: BTreeMap<ActionPatternId, &'a ActionPattern>,
    pub predicates: BTreeMap<PredicateId, &'a BoundPredicateGraph>,
    pub layouts: BTreeMap<BindingLayoutId, &'a BindingLayout>,
    pub trace_monitors: BTreeMap<MonitorTemplateId, &'a CompiledTraceAlternatives>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResidualReferenceLimits {
    pub max_obligations: usize,
    pub max_binding_checks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidualReferenceError {
    ObligationLimit,
    BindingCheckLimit,
    MissingPattern(ActionPatternId),
    MissingPredicate(PredicateId),
    MissingLayout(BindingLayoutId),
    LayoutIdentityMismatch(BindingLayoutId),
    MissingTraceMonitor(MonitorTemplateId),
    TraceMonitorIdentityMismatch(MonitorTemplateId),
    UnsupportedKind(ObligationId),
    InvalidBinding {
        obligation: ObligationId,
        reason: ResidualBindingError,
    },
}

/// Checks supplied links under one shared work budget. Trace template links
/// are checked, but site coverage and existing guards need further validators;
/// none of these results authorizes an effect.
pub fn validate_references(
    obligations: &ResidualObligationSet,
    catalog: &ResidualReferenceCatalog<'_>,
    limits: ResidualReferenceLimits,
) -> Result<(), ResidualReferenceError> {
    enum CheckedReference<'a> {
        Action(&'a ActionPattern, &'a BoundPredicateGraph),
        Trace,
    }
    if obligations.len() > limits.max_obligations {
        return Err(ResidualReferenceError::ObligationLimit);
    }
    let mut binding_checks = 0usize;
    for obligation in obligations.iter() {
        let layout_id = obligation.bindings();
        let reference = match obligation.kind() {
            ResidualKind::ActionPattern { pattern, predicate } => CheckedReference::Action(
                *catalog
                    .patterns
                    .get(pattern)
                    .ok_or(ResidualReferenceError::MissingPattern(*pattern))?,
                *catalog
                    .predicates
                    .get(predicate)
                    .ok_or(ResidualReferenceError::MissingPredicate(*predicate))?,
            ),
            ResidualKind::TraceContract { monitor } => {
                let template = catalog
                    .trace_monitors
                    .get(monitor)
                    .ok_or(ResidualReferenceError::MissingTraceMonitor(*monitor))?;
                if template.template_id() != *monitor {
                    return Err(ResidualReferenceError::TraceMonitorIdentityMismatch(
                        *monitor,
                    ));
                }
                CheckedReference::Trace
            }
            ResidualKind::ExistingGuard { .. } => {
                return Err(ResidualReferenceError::UnsupportedKind(obligation.id()));
            }
        };
        let layout = catalog
            .layouts
            .get(&layout_id)
            .ok_or(ResidualReferenceError::MissingLayout(layout_id))?;
        if layout.id() != layout_id {
            return Err(ResidualReferenceError::LayoutIdentityMismatch(layout_id));
        }
        let checks = match &reference {
            CheckedReference::Action(_, predicate) => predicate.binding_count(),
            CheckedReference::Trace => layout.len(),
        };
        binding_checks = binding_checks
            .checked_add(checks)
            .ok_or(ResidualReferenceError::BindingCheckLimit)?;
        if binding_checks > limits.max_binding_checks {
            return Err(ResidualReferenceError::BindingCheckLimit);
        }
        match reference {
            CheckedReference::Action(pattern, predicate) => {
                obligation
                    .validate_action_pattern_binding(pattern, predicate, layout)
                    .map_err(|reason| ResidualReferenceError::InvalidBinding {
                        obligation: obligation.id(),
                        reason,
                    })?;
            }
            CheckedReference::Trace => {}
        }
    }
    Ok(())
}
