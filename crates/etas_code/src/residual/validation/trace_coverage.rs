//! Coverage of exact trace components only, not plan installation or backend support.

use std::collections::{BTreeMap, BTreeSet};

use crate::residual::{
    identity::{MonitorTemplateId, ObligationId},
    model::{ResidualKind, ResidualObligationSet},
    monitor::CompiledTraceComponent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceCoverageEntry {
    pub obligation: ObligationId,
    pub component: MonitorTemplateId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceCoverageLimits {
    pub max_obligations: usize,
    pub max_components: usize,
    pub max_mappings: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraceCoverageError {
    NoObligations,
    ObligationLimit,
    ComponentLimit,
    MappingLimit,
    UnsupportedKind(ObligationId),
    MissingObligation(ObligationId),
    UnexpectedObligation(ObligationId),
    DuplicateObligation(ObligationId),
    MissingComponent(MonitorTemplateId),
    ComponentIdentityMismatch(MonitorTemplateId),
    IncompatibleComponent(ObligationId),
    UnusedComponent(MonitorTemplateId),
}

/// A borrowed coverage result does not establish native site bindings.
pub struct TraceComponentCoverage<'a> {
    entries: BTreeMap<ObligationId, &'a CompiledTraceComponent>,
}

impl<'a> TraceComponentCoverage<'a> {
    pub fn get(&self, id: ObligationId) -> Option<&'a CompiledTraceComponent> {
        self.entries.get(&id).copied()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn verify_trace_component_coverage<'a>(
    required: &ResidualObligationSet,
    components: &BTreeMap<MonitorTemplateId, &'a CompiledTraceComponent>,
    mappings: &[TraceCoverageEntry],
    limits: TraceCoverageLimits,
) -> Result<TraceComponentCoverage<'a>, TraceCoverageError> {
    if required.is_empty() {
        return Err(TraceCoverageError::NoObligations);
    }
    if required.len() > limits.max_obligations {
        return Err(TraceCoverageError::ObligationLimit);
    }
    if components.len() > limits.max_components {
        return Err(TraceCoverageError::ComponentLimit);
    }
    if mappings.len() > limits.max_mappings {
        return Err(TraceCoverageError::MappingLimit);
    }
    for obligation in required.iter() {
        if !matches!(obligation.kind(), ResidualKind::TraceContract { .. }) {
            return Err(TraceCoverageError::UnsupportedKind(obligation.id()));
        }
    }
    for (id, component) in components {
        if *id != component.id() {
            return Err(TraceCoverageError::ComponentIdentityMismatch(*id));
        }
    }
    let mut covered = BTreeMap::new();
    let mut used = BTreeSet::new();
    for mapping in mappings {
        let obligation = required
            .get(mapping.obligation)
            .ok_or(TraceCoverageError::UnexpectedObligation(mapping.obligation))?;
        let component = components
            .get(&mapping.component)
            .copied()
            .ok_or(TraceCoverageError::MissingComponent(mapping.component))?;
        if obligation != component.obligation() {
            return Err(TraceCoverageError::IncompatibleComponent(
                mapping.obligation,
            ));
        }
        if covered.insert(mapping.obligation, component).is_some() {
            return Err(TraceCoverageError::DuplicateObligation(mapping.obligation));
        }
        used.insert(mapping.component);
    }
    for obligation in required.iter() {
        if !covered.contains_key(&obligation.id()) {
            return Err(TraceCoverageError::MissingObligation(obligation.id()));
        }
    }
    for id in components.keys() {
        if !used.contains(id) {
            return Err(TraceCoverageError::UnusedComponent(*id));
        }
    }
    Ok(TraceComponentCoverage { entries: covered })
}
