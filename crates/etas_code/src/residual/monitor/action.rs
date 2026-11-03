//! A pre-dispatch predicate component. This is not a complete monitor plan,
//! backend capability declaration, site binding, installation proof or permit.

use std::collections::BTreeMap;

use super::super::{
    binding::{BindingLayout, BindingLayoutError, BindingLayoutLimits, BindingRoot},
    identity::{
        ActivationSiteId, MonitorTemplateId, ObligationIdentityError, ObligationIdentityLimits,
        SemanticScopeId, ValueBindingId,
    },
    model::{
        ActionPredicateDecision, ActionPredicateError, ActionPredicateInput, ResidualBindingError,
        ResidualObligation,
    },
    pattern::{ActionPattern, ActionPatternError, ActionPatternLimits},
    predicate::{
        BorrowedPredicateSupportError, BorrowedValue, BoundPredicateGraph, PredicateError,
        PredicateEvaluationLimits, PredicateLimits,
    },
};
use crate::{SourceRevisionKey, contract::CanonicalEffect, identity::canonical::CanonicalEncoder};

const COMPONENT_SCHEMA_VERSION: u32 = 2;
const COMPONENT_SEMANTIC_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub struct ActionComponentLimits {
    pub identity: ObligationIdentityLimits,
    pub pattern: ActionPatternLimits,
    pub predicate: PredicateLimits,
    pub layout: BindingLayoutLimits,
    pub evaluation: PredicateEvaluationLimits,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionComponentError {
    Identity(ObligationIdentityError),
    Pattern(ActionPatternError),
    Predicate(PredicateError),
    Layout(BindingLayoutError),
    Binding(ResidualBindingError),
    UnsupportedPredicate(BorrowedPredicateSupportError),
    UnsupportedBinding(ValueBindingId),
    LimitEncoding,
    IdentityMismatch,
}

pub struct CompiledActionPredicate {
    id: MonitorTemplateId,
    obligation: ResidualObligation,
    pattern: ActionPattern,
    predicate: BoundPredicateGraph,
    layout: BindingLayout,
    limits: ActionComponentLimits,
}

/// The owner scope and activation come from the checked native binding. This
/// data structure alone does not establish their runtime authenticity.
#[derive(Clone, Copy)]
pub struct ActionComponentInput<'a> {
    pub scope: SemanticScopeId,
    pub activation: ActivationSiteId,
    pub source_revision: SourceRevisionKey,
    pub action: &'a CanonicalEffect,
    pub bindings: &'a BTreeMap<ValueBindingId, BorrowedValue<'a>>,
}

impl CompiledActionPredicate {
    pub fn compile(
        obligation: ResidualObligation,
        pattern: ActionPattern,
        predicate: BoundPredicateGraph,
        layout: BindingLayout,
        limits: ActionComponentLimits,
    ) -> Result<Self, ActionComponentError> {
        obligation
            .id()
            .validate(obligation.identity(), limits.identity)
            .map_err(ActionComponentError::Identity)?;
        pattern
            .validate_with_limits(limits.pattern)
            .map_err(ActionComponentError::Pattern)?;
        predicate
            .validate_with_limits(limits.predicate)
            .map_err(ActionComponentError::Predicate)?;
        layout
            .validate_with_limits(limits.layout)
            .map_err(ActionComponentError::Layout)?;
        obligation
            .validate_action_pattern_binding(&pattern, &predicate, &layout)
            .map_err(ActionComponentError::Binding)?;
        if layout.len() > limits.evaluation.max_bindings {
            return Err(ActionComponentError::UnsupportedPredicate(
                BorrowedPredicateSupportError::BindingLimit,
            ));
        }
        predicate
            .validate_borrowed_support(limits.evaluation)
            .map_err(ActionComponentError::UnsupportedPredicate)?;
        for (id, binding) in layout.iter() {
            // This component runs before dispatch. Results do not exist yet;
            // evidence registers need a trusted provider adapter and are not
            // accepted by the borrowed-parameter evaluator.
            if !matches!(binding.root, BindingRoot::Parameter { .. }) {
                return Err(ActionComponentError::UnsupportedBinding(id));
            }
        }
        let id = component_id(&obligation, &pattern, &predicate, &layout, limits)?;
        Ok(Self {
            id,
            obligation,
            pattern,
            predicate,
            layout,
            limits,
        })
    }

    pub fn from_parts(
        id: MonitorTemplateId,
        obligation: ResidualObligation,
        pattern: ActionPattern,
        predicate: BoundPredicateGraph,
        layout: BindingLayout,
        limits: ActionComponentLimits,
    ) -> Result<Self, ActionComponentError> {
        let component = Self::compile(obligation, pattern, predicate, layout, limits)?;
        if component.id != id {
            return Err(ActionComponentError::IdentityMismatch);
        }
        Ok(component)
    }

    pub fn id(&self) -> MonitorTemplateId {
        self.id
    }
    pub fn obligation(&self) -> &ResidualObligation {
        &self.obligation
    }

    pub fn evaluate(
        &self,
        input: ActionComponentInput<'_>,
    ) -> Result<ActionPredicateDecision, ActionPredicateError> {
        self.obligation
            .evaluate_action_pattern(ActionPredicateInput {
                scope: input.scope,
                activation: input.activation,
                source_revision: input.source_revision,
                pattern: &self.pattern,
                predicate: &self.predicate,
                layout: &self.layout,
                action: input.action,
                bindings: input.bindings,
                pattern_limits: self.limits.pattern,
                evaluation_limits: self.limits.evaluation,
            })
    }
}

fn component_id(
    obligation: &ResidualObligation,
    pattern: &ActionPattern,
    predicate: &BoundPredicateGraph,
    layout: &BindingLayout,
    limits: ActionComponentLimits,
) -> Result<MonitorTemplateId, ActionComponentError> {
    let mut encoder = CanonicalEncoder::new("etas.code.residual.action-component.v1");
    encoder.number(u64::from(COMPONENT_SCHEMA_VERSION));
    encoder.number(u64::from(COMPONENT_SEMANTIC_VERSION));
    encoder.fingerprint(obligation.id().fingerprint());
    encoder.fingerprint(pattern.id().fingerprint());
    encoder.fingerprint(predicate.graph().root.fingerprint());
    encoder.fingerprint(layout.id().fingerprint());
    for limit in [
        limits.identity.max_generic_arguments,
        limits.identity.max_structural_site_depth,
        limits.pattern.max_selectors,
        limits.pattern.max_identity_segments,
        limits.pattern.max_identity_bytes,
        limits.pattern.max_literal_bytes,
        limits.predicate.max_nodes,
        limits.predicate.max_edges,
        limits.predicate.max_depth,
        limits.predicate.max_evidence_arguments,
        limits.layout.max_bindings,
        limits.layout.max_projections_per_binding,
        limits.layout.max_total_projections,
        limits.evaluation.max_nodes,
        limits.evaluation.max_bindings,
        limits.evaluation.max_work,
        limits.evaluation.max_value_bytes,
        limits.evaluation.max_comparison_bytes,
    ] {
        encoder.number(u64::try_from(limit).map_err(|_| ActionComponentError::LimitEncoding)?);
    }
    Ok(MonitorTemplateId::from_canonical_fingerprint(
        encoder.finish(),
    ))
}
