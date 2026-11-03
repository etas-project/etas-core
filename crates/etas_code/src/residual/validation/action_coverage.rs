//! Checks one restricted component family. This is not full plan coverage,
//! backend support, site binding, or installation evidence.

use std::collections::BTreeMap;

use crate::residual::{
    identity::{MonitorTemplateId, ObligationId},
    model::{ResidualKind, ResidualObligation, ResidualObligationSet},
    monitor::CompiledActionPredicate,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionCoverageEntry {
    pub obligation: ObligationId,
    pub component: MonitorTemplateId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionCoverageLimits {
    pub max_obligations: usize,
    pub max_components: usize,
    pub max_mappings: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionCoverageError {
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

/// The returned view is valid only while its compiled components remain alive.
/// It cannot be serialized or used as runtime installation proof.
pub struct ActionComponentCoverage<'a> {
    entries: BTreeMap<ObligationId, &'a CompiledActionPredicate>,
}

impl<'a> ActionComponentCoverage<'a> {
    pub fn get(&self, id: ObligationId) -> Option<&'a CompiledActionPredicate> {
        self.entries.get(&id).copied()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn verify_action_component_coverage<'a>(
    required: &ResidualObligationSet,
    components: &BTreeMap<MonitorTemplateId, &'a CompiledActionPredicate>,
    mappings: &[ActionCoverageEntry],
    limits: ActionCoverageLimits,
) -> Result<ActionComponentCoverage<'a>, ActionCoverageError> {
    if required.is_empty() {
        return Err(ActionCoverageError::NoObligations);
    }
    if required.len() > limits.max_obligations {
        return Err(ActionCoverageError::ObligationLimit);
    }
    if components.len() > limits.max_components {
        return Err(ActionCoverageError::ComponentLimit);
    }
    if mappings.len() > limits.max_mappings {
        return Err(ActionCoverageError::MappingLimit);
    }
    for obligation in required.iter() {
        if !matches!(obligation.kind(), ResidualKind::ActionPattern { .. }) {
            return Err(ActionCoverageError::UnsupportedKind(obligation.id()));
        }
    }
    for (id, component) in components {
        if *id != component.id() {
            return Err(ActionCoverageError::ComponentIdentityMismatch(*id));
        }
    }
    let mut covered = BTreeMap::new();
    let mut used = std::collections::BTreeSet::new();
    for mapping in mappings {
        let obligation =
            required
                .get(mapping.obligation)
                .ok_or(ActionCoverageError::UnexpectedObligation(
                    mapping.obligation,
                ))?;
        let component = components
            .get(&mapping.component)
            .copied()
            .ok_or(ActionCoverageError::MissingComponent(mapping.component))?;
        if !compatible(obligation, component.obligation()) {
            return Err(ActionCoverageError::IncompatibleComponent(
                mapping.obligation,
            ));
        }
        if covered.insert(mapping.obligation, component).is_some() {
            return Err(ActionCoverageError::DuplicateObligation(mapping.obligation));
        }
        used.insert(mapping.component);
    }
    for obligation in required.iter() {
        if !covered.contains_key(&obligation.id()) {
            return Err(ActionCoverageError::MissingObligation(obligation.id()));
        }
    }
    for id in components.keys() {
        if !used.contains(id) {
            return Err(ActionCoverageError::UnusedComponent(*id));
        }
    }
    Ok(ActionComponentCoverage { entries: covered })
}

fn compatible(required: &ResidualObligation, component: &ResidualObligation) -> bool {
    // Different clauses may assign different meaning to the same predicate.
    // Sharing needs a checked clause/phase coverage proof, not shape equality.
    required == component
}
