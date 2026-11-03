//! Descriptive residual obligations, not runtime installation evidence.

use std::collections::BTreeMap;

use super::identity::{
    ActionPatternId, ActivationSiteId, BindingLayoutId, GuardProviderId, GuardRequirementId,
    MonitorTemplateId, ObligationId, ObligationIdentityError, ObligationIdentityInputs,
    ObligationIdentityLimits, PredicateId, ResidualRuleKind, SemanticScopeId,
};
use super::{
    binding::{BindingLayout, BorrowedBindingError, BorrowedBindingLimits},
    pattern::{ActionPattern, ActionPatternError, ActionPatternLimits, ActionPatternMatch},
    predicate::{
        BorrowedValue, BoundPredicateGraph, PredicateDecision, PredicateError,
        PredicateEvaluationError, PredicateEvaluationLimits,
    },
};

/// Inputs are descriptive checked data supplied by the native site adapter.
/// Neither these inputs nor the result are an installation proof or permit.
pub struct ActionPredicateInput<'a> {
    /// The active plan owner's lexical scope, not the current callee's scope.
    pub scope: SemanticScopeId,
    pub activation: ActivationSiteId,
    /// The active scope owner's source revision; the callee may differ.
    pub source_revision: crate::SourceRevisionKey,
    pub pattern: &'a ActionPattern,
    pub predicate: &'a BoundPredicateGraph,
    pub layout: &'a BindingLayout,
    pub action: &'a crate::contract::CanonicalEffect,
    pub bindings: &'a BTreeMap<super::identity::ValueBindingId, BorrowedValue<'a>>,
    pub pattern_limits: ActionPatternLimits,
    pub evaluation_limits: PredicateEvaluationLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionPredicateDecision {
    NotApplicable,
    NeedsConcreteSelector,
    Applicable(PredicateDecision),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionPredicateError {
    ScopeMismatch,
    ActivationMismatch,
    RevisionMismatch,
    InvalidBinding(ResidualBindingError),
    InvalidAction(ActionPatternError),
    Evaluation(PredicateEvaluationError),
}

#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub enum ResidualKind {
    ActionPattern {
        pattern: ActionPatternId,
        predicate: PredicateId,
    },
    TraceContract {
        monitor: MonitorTemplateId,
    },
    ExistingGuard {
        provider: GuardProviderId,
        requirement: GuardRequirementId,
    },
}

impl ResidualKind {
    fn rule_kind(&self) -> ResidualRuleKind {
        match self {
            Self::ActionPattern { .. } => ResidualRuleKind::ActionPattern,
            Self::TraceContract { .. } => ResidualRuleKind::TraceContract,
            Self::ExistingGuard { .. } => ResidualRuleKind::ExistingGuard,
        }
    }
}

/// The identity and its inputs are kept together so import cannot silently
/// associate a valid key with a different scope or generic instantiation.
#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ResidualObligation {
    id: ObligationId,
    identity: ObligationIdentityInputs,
    kind: ResidualKind,
    bindings: BindingLayoutId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidualBindingError {
    NotActionPattern,
    ActionPatternMismatch,
    PredicateMismatch,
    LayoutMismatch,
    Predicate(PredicateError),
}

impl ResidualObligation {
    pub fn new(
        identity: ObligationIdentityInputs,
        kind: ResidualKind,
        bindings: BindingLayoutId,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        if identity.rule.kind() != kind.rule_kind() {
            return Err(ObligationIdentityError::RuleKindMismatch);
        }
        let id = ObligationId::from_inputs(&identity, limits)?;
        Ok(Self {
            id,
            identity,
            kind,
            bindings,
        })
    }

    /// Reconstruct descriptive data only after checking the claimed identity.
    /// Artifact limits, graph references and installation still need validation.
    pub fn from_parts(
        id: ObligationId,
        identity: ObligationIdentityInputs,
        kind: ResidualKind,
        bindings: BindingLayoutId,
        limits: ObligationIdentityLimits,
    ) -> Result<Self, ObligationIdentityError> {
        if identity.rule.kind() != kind.rule_kind() {
            return Err(ObligationIdentityError::RuleKindMismatch);
        }
        id.validate(&identity, limits)?;
        Ok(Self {
            id,
            identity,
            kind,
            bindings,
        })
    }

    pub fn id(&self) -> ObligationId {
        self.id
    }

    pub fn identity(&self) -> &ObligationIdentityInputs {
        &self.identity
    }

    pub fn kind(&self) -> &ResidualKind {
        &self.kind
    }

    pub fn bindings(&self) -> BindingLayoutId {
        self.bindings
    }

    /// Validates only graph/layout linkage; plan coverage, provider support and
    /// runtime installation remain separate mandatory checks.
    pub fn validate_predicate_binding(
        &self,
        predicate: &BoundPredicateGraph,
        layout: &BindingLayout,
    ) -> Result<(), ResidualBindingError> {
        let ResidualKind::ActionPattern {
            predicate: expected,
            ..
        } = self.kind
        else {
            return Err(ResidualBindingError::NotActionPattern);
        };
        if expected != predicate.graph().root {
            return Err(ResidualBindingError::PredicateMismatch);
        }
        if self.bindings != layout.id() {
            return Err(ResidualBindingError::LayoutMismatch);
        }
        predicate
            .validate_layout(layout)
            .map_err(ResidualBindingError::Predicate)
    }

    /// Checks descriptive action, predicate and layout linkage only.
    pub fn validate_action_pattern_binding(
        &self,
        pattern: &ActionPattern,
        predicate: &BoundPredicateGraph,
        layout: &BindingLayout,
    ) -> Result<(), ResidualBindingError> {
        let ResidualKind::ActionPattern {
            pattern: expected, ..
        } = self.kind
        else {
            return Err(ResidualBindingError::NotActionPattern);
        };
        if expected != pattern.id() {
            return Err(ResidualBindingError::ActionPatternMismatch);
        }
        self.validate_predicate_binding(predicate, layout)
    }

    /// Evaluates an action instance against this obligation's checked pattern
    /// and predicate. Partial selector overlap requires concrete evidence and
    /// is never interpreted as an unrelated action.
    pub fn evaluate_action_pattern(
        &self,
        input: ActionPredicateInput<'_>,
    ) -> Result<ActionPredicateDecision, ActionPredicateError> {
        if input.scope != self.identity.scope {
            return Err(ActionPredicateError::ScopeMismatch);
        }
        if input.activation != self.identity.activation {
            return Err(ActionPredicateError::ActivationMismatch);
        }
        if input.source_revision != self.identity.origin.revision {
            return Err(ActionPredicateError::RevisionMismatch);
        }
        if input.predicate.graph().nodes.len() > input.evaluation_limits.max_nodes {
            return Err(ActionPredicateError::Evaluation(
                PredicateEvaluationError::NodeLimit,
            ));
        }
        if input.predicate.binding_count() > input.evaluation_limits.max_bindings
            || input.bindings.len() > input.evaluation_limits.max_bindings
        {
            return Err(ActionPredicateError::Evaluation(
                PredicateEvaluationError::BindingLimit,
            ));
        }
        self.validate_action_pattern_binding(input.pattern, input.predicate, input.layout)
            .map_err(ActionPredicateError::InvalidBinding)?;
        match input
            .pattern
            .match_action(input.action, input.pattern_limits)
            .map_err(ActionPredicateError::InvalidAction)?
        {
            ActionPatternMatch::Disjoint => return Ok(ActionPredicateDecision::NotApplicable),
            ActionPatternMatch::NeedsConcreteSelector => {
                return Ok(ActionPredicateDecision::NeedsConcreteSelector);
            }
            ActionPatternMatch::Matches => {}
        }
        let bindings = input
            .layout
            .validate_borrowed(
                input.bindings,
                BorrowedBindingLimits {
                    max_bindings: input.evaluation_limits.max_bindings,
                    max_value_bytes: input.evaluation_limits.max_value_bytes,
                    max_total_value_bytes: input
                        .evaluation_limits
                        .max_value_bytes
                        .saturating_mul(input.evaluation_limits.max_bindings),
                },
            )
            .map_err(|error| {
                ActionPredicateError::Evaluation(match error {
                    BorrowedBindingError::BindingLimit => PredicateEvaluationError::BindingLimit,
                    BorrowedBindingError::TypeMismatch(id) => {
                        PredicateEvaluationError::TypeMismatch(id)
                    }
                    BorrowedBindingError::ValueSizeLimit(_)
                    | BorrowedBindingError::TotalValueSizeLimit => {
                        PredicateEvaluationError::ValueSizeLimit
                    }
                })
            })?;
        input
            .predicate
            .evaluate_validated(&bindings, input.evaluation_limits)
            .map(ActionPredicateDecision::Applicable)
            .map_err(ActionPredicateError::Evaluation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResidualSetError {
    ConflictingObligation(ObligationId),
    InvalidIdentity(ObligationIdentityError),
    ObligationLimit,
    GenericArgumentLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ResidualSetLimits {
    pub identity: ObligationIdentityLimits,
    pub max_obligations: usize,
    pub max_generic_arguments: usize,
}

/// Stable identities deduplicate the same obligation, not its human label.
#[derive(Clone, Debug, PartialEq, Eq, etas_core::RetainedSize)]
pub struct ResidualObligationSet {
    obligations: BTreeMap<ObligationId, ResidualObligation>,
    total_generic_arguments: usize,
    limits: ResidualSetLimits,
}

impl ResidualObligationSet {
    pub fn new(limits: ResidualSetLimits) -> Self {
        Self {
            obligations: BTreeMap::new(),
            total_generic_arguments: 0,
            limits,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.obligations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.obligations.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ResidualObligation> {
        self.obligations.values()
    }

    pub fn get(&self, id: ObligationId) -> Option<&ResidualObligation> {
        self.obligations.get(&id)
    }

    pub fn insert(&mut self, obligation: ResidualObligation) -> Result<bool, ResidualSetError> {
        obligation
            .id
            .validate(&obligation.identity, self.limits.identity)
            .map_err(ResidualSetError::InvalidIdentity)?;
        if let Some(existing) = self.obligations.get(&obligation.id) {
            return if existing == &obligation {
                Ok(false)
            } else {
                Err(ResidualSetError::ConflictingObligation(obligation.id))
            };
        }
        if self.obligations.len() >= self.limits.max_obligations {
            return Err(ResidualSetError::ObligationLimit);
        }
        let total = self
            .total_generic_arguments
            .checked_add(obligation.identity.static_bindings.len())
            .ok_or(ResidualSetError::GenericArgumentLimit)?;
        if total > self.limits.max_generic_arguments {
            return Err(ResidualSetError::GenericArgumentLimit);
        }
        self.obligations.insert(obligation.id, obligation);
        self.total_generic_arguments = total;
        Ok(true)
    }

    /// A failed merge leaves the original set unchanged.
    pub fn union_assign(&mut self, other: &Self) -> Result<bool, ResidualSetError> {
        let mut count = self.obligations.len();
        let mut generic_arguments = self.total_generic_arguments;
        for (id, obligation) in &other.obligations {
            id.validate(&obligation.identity, self.limits.identity)
                .map_err(ResidualSetError::InvalidIdentity)?;
            if self
                .obligations
                .get(id)
                .is_some_and(|existing| existing != obligation)
            {
                return Err(ResidualSetError::ConflictingObligation(*id));
            }
            if !self.obligations.contains_key(id) {
                count = count
                    .checked_add(1)
                    .ok_or(ResidualSetError::ObligationLimit)?;
                if count > self.limits.max_obligations {
                    return Err(ResidualSetError::ObligationLimit);
                }
                generic_arguments = generic_arguments
                    .checked_add(obligation.identity.static_bindings.len())
                    .ok_or(ResidualSetError::GenericArgumentLimit)?;
                if generic_arguments > self.limits.max_generic_arguments {
                    return Err(ResidualSetError::GenericArgumentLimit);
                }
            }
        }
        let old_len = self.obligations.len();
        self.obligations.extend(
            other
                .obligations
                .iter()
                .map(|(id, obligation)| (*id, obligation.clone())),
        );
        self.total_generic_arguments = generic_arguments;
        Ok(self.obligations.len() != old_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey, SourceRevisionKey,
        StableSourceKey, StableSourceOrigin,
        contract::{CanonicalEffect, CanonicalSelector},
        descriptor::EffectTargetInfo,
        residual::identity::{
            ActivationSiteId, BehaviorContractRef, ClauseId, GenericParameterId, ResidualOwnerId,
            ResidualOwnerKind, ResidualRuleKey, SemanticScopeId, SourceRevisionLocation,
            SourceStructuralSite, SpecInstanceId, StaticBindingRecipe,
        },
        residual::{
            binding::{BindingDescriptor, BindingLayout, BindingLayoutLimits, BindingRoot},
            monitor::{
                ActionComponentError, ActionComponentInput, ActionComponentLimits,
                CompiledActionPredicate,
            },
            pattern::{ActionPattern, ActionPatternLimits},
            predicate::{
                BorrowedValue, PredicateDecision, PredicateEvaluationLimits, PredicateGraph,
                PredicateLimits, PredicateNode, PredicateSchema, ValueType,
            },
            validation::{
                ActionCoverageEntry, ActionCoverageError, ActionCoverageLimits,
                ResidualReferenceCatalog, ResidualReferenceError, ResidualReferenceLimits,
                validate_references, verify_action_component_coverage,
            },
        },
    };

    fn fp(byte: u8) -> CodeFingerprint {
        CodeFingerprint::from_bytes([byte; 32])
    }

    fn limits() -> ResidualSetLimits {
        ResidualSetLimits {
            identity: ObligationIdentityLimits {
                max_generic_arguments: 8,
                max_structural_site_depth: 8,
            },
            max_obligations: 8,
            max_generic_arguments: 16,
        }
    }

    fn obligation(scope: u8, predicate: u8) -> ResidualObligation {
        let source =
            StableSourceKey::new(StableSourceOrigin::Project(fp(2)), &["main.es".into()]).unwrap();
        let owner_location = SourceRevisionLocation::new(source, b"source bytes", 0, 12).unwrap();
        let identity = ObligationIdentityInputs {
            origin: SourceRevisionLocation {
                source,
                revision: SourceRevisionKey::new(source, b"source bytes"),
                byte_start: 4,
                byte_end: 8,
            },
            owner: ResidualOwnerId::from_source_declaration(
                &owner_location,
                b"source bytes",
                ResidualOwnerKind::Flow,
            )
            .unwrap(),
            contract: BehaviorContractRef::SourceConstraint,
            spec: SpecInstanceId::from_canonical_fingerprint(fp(5)),
            clause: ClauseId::from_canonical_fingerprint(fp(6)),
            rule: ResidualRuleKey::from_source_site(
                &SourceStructuralSite {
                    location: &owner_location,
                    source_bytes: b"source bytes",
                    structural_path: &[],
                },
                ResidualRuleKind::ActionPattern,
                limits().identity,
            )
            .unwrap(),
            scope: SemanticScopeId::from_canonical_fingerprint(fp(scope)),
            activation: ActivationSiteId::from_canonical_fingerprint(fp(8)),
            static_bindings: BTreeMap::new(),
        };
        ResidualObligation::new(
            identity,
            ResidualKind::ActionPattern {
                pattern: ActionPatternId::from_canonical_fingerprint(fp(9)),
                predicate: PredicateId::from_canonical_fingerprint(fp(predicate)),
            },
            BindingLayoutId::from_canonical_fingerprint(fp(11)),
            limits().identity,
        )
        .unwrap()
    }

    fn identity_for_rule(
        obligation: &ResidualObligation,
        kind: ResidualRuleKind,
    ) -> ObligationIdentityInputs {
        let mut identity = obligation.identity().clone();
        identity.rule = ResidualRuleKey::from_source_site(
            &SourceStructuralSite {
                location: &identity.origin,
                source_bytes: b"source bytes",
                structural_path: &[],
            },
            kind,
            limits().identity,
        )
        .unwrap();
        identity
    }

    #[test]
    fn reconstruct_rejects_a_scope_changed_under_an_existing_identity() {
        let original = obligation(7, 10);
        let altered = obligation(12, 10);
        assert_eq!(
            ResidualObligation::from_parts(
                original.id(),
                altered.identity().clone(),
                altered.kind().clone(),
                altered.bindings(),
                limits().identity,
            ),
            Err(ObligationIdentityError::IdentityMismatch)
        );
    }

    #[test]
    fn obligation_rejects_a_kind_that_does_not_match_its_source_rule() {
        let original = obligation(7, 10);
        let wrong_kind = ResidualKind::TraceContract {
            monitor: MonitorTemplateId::from_canonical_fingerprint(fp(12)),
        };
        assert_eq!(
            ResidualObligation::new(
                original.identity().clone(),
                wrong_kind.clone(),
                original.bindings(),
                limits().identity,
            ),
            Err(ObligationIdentityError::RuleKindMismatch)
        );
        assert_eq!(
            ResidualObligation::from_parts(
                original.id(),
                original.identity().clone(),
                wrong_kind,
                original.bindings(),
                limits().identity,
            ),
            Err(ObligationIdentityError::RuleKindMismatch)
        );
    }

    #[test]
    fn union_preserves_distinct_scopes_and_deduplicates_identical_obligations() {
        let first = obligation(7, 10);
        let nested = obligation(12, 10);
        let mut set = ResidualObligationSet::new(limits());
        assert_eq!(set.insert(first.clone()), Ok(true));
        assert_eq!(set.insert(first), Ok(false));
        assert_eq!(set.insert(nested), Ok(true));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn conflicting_identity_rejects_merge_without_partial_update() {
        let first = obligation(7, 10);
        let conflicting = obligation(7, 13);
        let new_scope = obligation(12, 10);
        let mut original = ResidualObligationSet::new(limits());
        original.insert(first.clone()).unwrap();
        let before = original.clone();
        let mut incoming = ResidualObligationSet::new(limits());
        incoming.insert(new_scope).unwrap();
        incoming.insert(conflicting).unwrap();
        assert_eq!(
            original.union_assign(&incoming),
            Err(ResidualSetError::ConflictingObligation(first.id()))
        );
        assert_eq!(original, before);
    }

    #[test]
    fn merge_obeys_destination_limits_without_partial_update() {
        let first = obligation(7, 10);
        let nested = obligation(12, 10);
        let mut strict = limits();
        strict.max_obligations = 1;
        let mut original = ResidualObligationSet::new(strict);
        original.insert(first).unwrap();
        let before = original.clone();
        let mut incoming = ResidualObligationSet::new(limits());
        incoming.insert(nested).unwrap();
        assert_eq!(
            original.union_assign(&incoming),
            Err(ResidualSetError::ObligationLimit)
        );
        assert_eq!(original, before);
    }

    #[test]
    fn merge_enforces_aggregate_generic_budget_and_destination_identity_limit() {
        let base = obligation(7, 10);
        let mut identity = base.identity().clone();
        identity.static_bindings.insert(
            GenericParameterId::from_canonical_fingerprint(fp(14)),
            StaticBindingRecipe::ExplicitType(fp(15)),
        );
        let generic = ResidualObligation::new(
            identity,
            base.kind().clone(),
            base.bindings(),
            limits().identity,
        )
        .unwrap();
        let mut incoming = ResidualObligationSet::new(limits());
        incoming.insert(generic).unwrap();
        let mut strict = limits();
        strict.max_generic_arguments = 0;
        let mut destination = ResidualObligationSet::new(strict);
        let before = destination.clone();
        assert_eq!(
            destination.union_assign(&incoming),
            Err(ResidualSetError::GenericArgumentLimit)
        );
        assert_eq!(destination, before);
        strict = limits();
        strict.identity.max_generic_arguments = 0;
        let mut destination = ResidualObligationSet::new(strict);
        let before = destination.clone();
        assert_eq!(
            destination.union_assign(&incoming),
            Err(ResidualSetError::InvalidIdentity(
                ObligationIdentityError::GenericArgumentLimit
            ))
        );
        assert_eq!(destination, before);
    }

    #[test]
    fn missing_monitor_reference_is_not_accepted_as_valid() {
        let base = obligation(7, 10);
        let item = ResidualObligation::new(
            identity_for_rule(&base, ResidualRuleKind::TraceContract),
            ResidualKind::TraceContract {
                monitor: MonitorTemplateId::from_canonical_fingerprint(fp(16)),
            },
            base.bindings(),
            limits().identity,
        )
        .unwrap();
        let mut obligations = ResidualObligationSet::new(limits());
        obligations.insert(item.clone()).unwrap();
        assert_eq!(
            validate_references(
                &obligations,
                &ResidualReferenceCatalog::default(),
                ResidualReferenceLimits {
                    max_obligations: 1,
                    max_binding_checks: 10
                }
            ),
            Err(ResidualReferenceError::MissingTraceMonitor(
                MonitorTemplateId::from_canonical_fingerprint(fp(16))
            ))
        );
    }

    #[test]
    fn trace_monitor_reference_requires_exact_checked_template_and_layout() {
        use crate::residual::monitor::{
            ActionAttemptId, AlternativeLimits, AlternativeRule, AlternativeScope,
            CompiledTraceAlternatives, CompiledTraceComponent, ConcreteActionAttempt,
            ConcreteEvidenceLimits, ConcretePriorEvidence, EvidenceQuery, EvidenceStatus,
            ScopeCheckResult, ScopeExit, ScopeInvocationId, SemanticEventCursor,
            TraceComponentDecision, TraceComponentError, TraceComponentLimits,
        };
        use crate::residual::validation::{
            TraceCoverageEntry, TraceCoverageError, TraceCoverageLimits,
            verify_trace_component_coverage,
        };

        let pattern_limits = ActionPatternLimits {
            max_selectors: 2,
            max_identity_segments: 8,
            max_identity_bytes: 128,
            max_literal_bytes: 64,
        };
        let pattern = ActionPattern::new(
            CanonicalEffect {
                target: EffectTargetInfo::Action {
                    effect: DefinitionKey::new(
                        PackageKey::from_canonical_fingerprint(fp(1)),
                        vec!["app".into()],
                        vec!["Gate".into()],
                        DefinitionKind::Effect,
                    )
                    .unwrap(),
                    name: "read".into(),
                },
                arguments: Vec::new(),
            },
            pattern_limits,
        )
        .unwrap();
        let monitor_limits = AlternativeLimits {
            evidence: ConcreteEvidenceLimits {
                pattern: pattern_limits,
                max_bindings: 2,
                max_value_bytes: 64,
            },
            max_alternatives: 2,
            max_clauses: 4,
            max_patterns: 4,
            max_checks_per_event: 8,
            max_product_work_per_event: 16,
        };
        let template = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Allow(pattern.clone())]],
            monitor_limits,
        )
        .unwrap();
        let descriptor = BindingDescriptor {
            root: BindingRoot::Parameter { position: 0 },
            fields: vec![],
            value_type: ValueType::Text,
        };
        let layout = BindingLayout::new(
            BTreeMap::from([(descriptor.id(1).unwrap(), descriptor)]),
            BindingLayoutLimits {
                max_bindings: 2,
                max_projections_per_binding: 1,
                max_total_projections: 1,
            },
        )
        .unwrap();
        let base = obligation(7, 10);
        let item = ResidualObligation::new(
            identity_for_rule(&base, ResidualRuleKind::TraceContract),
            ResidualKind::TraceContract {
                monitor: template.template_id(),
            },
            layout.id(),
            limits().identity,
        )
        .unwrap();
        let mut obligations = ResidualObligationSet::new(limits());
        obligations.insert(item.clone()).unwrap();
        let bounds = ResidualReferenceLimits {
            max_obligations: 1,
            max_binding_checks: 1,
        };
        let mut catalog = ResidualReferenceCatalog::default();
        assert_eq!(
            validate_references(&obligations, &catalog, bounds),
            Err(ResidualReferenceError::MissingTraceMonitor(
                template.template_id()
            ))
        );
        let wrong_key = MonitorTemplateId::from_canonical_fingerprint(fp(77));
        let wrong_item = ResidualObligation::new(
            identity_for_rule(&base, ResidualRuleKind::TraceContract),
            ResidualKind::TraceContract { monitor: wrong_key },
            layout.id(),
            limits().identity,
        )
        .unwrap();
        let mut wrong_set = ResidualObligationSet::new(limits());
        wrong_set.insert(wrong_item).unwrap();
        catalog.trace_monitors.insert(wrong_key, &template);
        assert_eq!(
            validate_references(&wrong_set, &catalog, bounds),
            Err(ResidualReferenceError::TraceMonitorIdentityMismatch(
                wrong_key
            ))
        );
        catalog
            .trace_monitors
            .insert(template.template_id(), &template);
        assert_eq!(
            validate_references(&obligations, &catalog, bounds),
            Err(ResidualReferenceError::MissingLayout(layout.id()))
        );
        let different_layout = BindingLayout::new(
            BTreeMap::new(),
            BindingLayoutLimits {
                max_bindings: 2,
                max_projections_per_binding: 1,
                max_total_projections: 1,
            },
        )
        .unwrap();
        catalog.layouts.insert(layout.id(), &different_layout);
        assert_eq!(
            validate_references(&obligations, &catalog, bounds),
            Err(ResidualReferenceError::LayoutIdentityMismatch(layout.id()))
        );
        catalog.layouts.insert(layout.id(), &layout);
        assert_eq!(validate_references(&obligations, &catalog, bounds), Ok(()));
        assert_eq!(
            validate_references(
                &obligations,
                &catalog,
                ResidualReferenceLimits {
                    max_binding_checks: 0,
                    ..bounds
                }
            ),
            Err(ResidualReferenceError::BindingCheckLimit)
        );

        let component_limits = TraceComponentLimits {
            identity: limits().identity,
            layout: BindingLayoutLimits {
                max_bindings: 2,
                max_projections_per_binding: 1,
                max_total_projections: 1,
            },
            monitor: monitor_limits,
        };
        let patterns = BTreeMap::from([(pattern.id(), pattern.clone())]);
        let component = CompiledTraceComponent::compile(
            item.clone(),
            template.template(),
            &patterns,
            &layout,
            component_limits,
        )
        .unwrap();
        let components = BTreeMap::from([(component.id(), &component)]);
        let mapping = TraceCoverageEntry {
            obligation: item.id(),
            component: component.id(),
        };
        let coverage_limits = TraceCoverageLimits {
            max_obligations: 2,
            max_components: 2,
            max_mappings: 2,
        };
        let coverage =
            verify_trace_component_coverage(&obligations, &components, &[mapping], coverage_limits)
                .unwrap();
        assert_eq!(coverage.len(), 1);
        assert_eq!(coverage.get(item.id()).unwrap().id(), component.id());
        assert!(
            matches!(verify_trace_component_coverage(&obligations, &components, &[], coverage_limits), Err(TraceCoverageError::MissingObligation(id)) if id == item.id())
        );
        assert!(
            matches!(verify_trace_component_coverage(&obligations, &components, &[mapping, mapping], coverage_limits), Err(TraceCoverageError::DuplicateObligation(id)) if id == item.id())
        );
        let mut other_scope = item.identity().clone();
        other_scope.scope = SemanticScopeId::from_canonical_fingerprint(fp(66));
        let other_item = ResidualObligation::new(
            other_scope,
            item.kind().clone(),
            item.bindings(),
            limits().identity,
        )
        .unwrap();
        let mut other_required = ResidualObligationSet::new(limits());
        other_required.insert(other_item.clone()).unwrap();
        assert!(
            matches!(verify_trace_component_coverage(&other_required, &components,
            &[TraceCoverageEntry { obligation: other_item.id(), component: component.id() }], coverage_limits),
            Err(TraceCoverageError::IncompatibleComponent(id)) if id == other_item.id())
        );
        assert!(
            matches!(verify_trace_component_coverage(&obligations, &components,
            &[TraceCoverageEntry { obligation: other_item.id(), component: component.id() }], coverage_limits),
            Err(TraceCoverageError::UnexpectedObligation(id)) if id == other_item.id())
        );
        assert!(
            matches!(verify_trace_component_coverage(&obligations, &components,
            &[TraceCoverageEntry { obligation: item.id(), component: wrong_key }], coverage_limits),
            Err(TraceCoverageError::MissingComponent(id)) if id == wrong_key)
        );
        assert!(matches!(verify_trace_component_coverage(&obligations,
            &BTreeMap::from([(wrong_key, &component)]), &[mapping], coverage_limits),
            Err(TraceCoverageError::ComponentIdentityMismatch(id)) if id == wrong_key));
        let other_component = CompiledTraceComponent::compile(
            other_item,
            template.template(),
            &patterns,
            &layout,
            component_limits,
        )
        .unwrap();
        let with_unused = BTreeMap::from([
            (component.id(), &component),
            (other_component.id(), &other_component),
        ]);
        assert!(
            matches!(verify_trace_component_coverage(&obligations, &with_unused, &[mapping], coverage_limits),
            Err(TraceCoverageError::UnusedComponent(id)) if id == other_component.id())
        );
        assert!(matches!(
            verify_trace_component_coverage(
                &obligations,
                &components,
                &[mapping],
                TraceCoverageLimits {
                    max_mappings: 0,
                    ..coverage_limits
                }
            ),
            Err(TraceCoverageError::MappingLimit)
        ));
        assert!(matches!(
            CompiledTraceComponent::from_parts(
                wrong_key,
                item.clone(),
                template.template(),
                &patterns,
                &layout,
                component_limits,
            ),
            Err(TraceComponentError::IdentityMismatch)
        ));
        assert!(matches!(
            CompiledTraceComponent::compile(
                base.clone(),
                template.template(),
                &patterns,
                &layout,
                component_limits,
            ),
            Err(TraceComponentError::LayoutMismatch) | Err(TraceComponentError::NotTraceContract)
        ));
        let scope = AlternativeScope {
            scope: item.identity().scope,
            activation: item.identity().activation,
            revision: crate::RevisionKey::from_canonical_fingerprint(fp(99)),
            invocation: ScopeInvocationId::from_bytes([1; 16]),
        };
        let mut wrong_scope = scope;
        wrong_scope.scope = SemanticScopeId::from_canonical_fingerprint(fp(88));
        assert!(matches!(
            component.initial_state(wrong_scope, item.identity().origin.revision),
            Err(TraceComponentError::ScopeMismatch)
        ));
        assert!(matches!(
            component.initial_state(
                scope,
                SourceRevisionKey::new(item.identity().origin.source, b"wrong")
            ),
            Err(TraceComponentError::SourceRevisionMismatch)
        ));
        let mut state = component
            .initial_state(scope, item.identity().origin.revision)
            .unwrap();
        let binding_id = layout.iter().next().unwrap().0;
        let values = BTreeMap::from([(binding_id, BorrowedValue::Text("value"))]);
        let binding_limits = crate::residual::binding::BorrowedBindingLimits {
            max_bindings: 2,
            max_value_bytes: 64,
            max_total_value_bytes: 128,
        };
        let attempt = ConcreteActionAttempt {
            scope: scope.scope,
            activation: scope.activation,
            revision: scope.revision,
            invocation: scope.invocation,
            id: ActionAttemptId::from_bytes([2; 16]),
            cursor: SemanticEventCursor(1),
            action: pattern.effect(),
            bindings: layout.validate_borrowed(&values, binding_limits).unwrap(),
        };
        struct NoEvidence;
        impl ConcretePriorEvidence for NoEvidence {
            fn query(&self, _: EvidenceQuery<'_, '_>) -> EvidenceStatus {
                EvidenceStatus::Unavailable
            }
        }
        {
            use crate::residual::monitor::{
                AlternativeError, TraceBatchCommitError, TraceComponentTransition,
                commit_trace_transitions,
            };
            let mut other_identity = item.identity().clone();
            other_identity.clause = ClauseId::from_canonical_fingerprint(fp(78));
            let other_item = ResidualObligation::new(
                other_identity,
                item.kind().clone(),
                layout.id(),
                limits().identity,
            )
            .unwrap();
            let other = CompiledTraceComponent::compile(
                other_item.clone(),
                template.template(),
                &patterns,
                &layout,
                component_limits,
            )
            .unwrap();
            {
                use crate::residual::plan::{
                    ResidualComponentAttempt, ResidualScopeDecision, ResidualScopeError,
                    ResidualScopeEvent, SemanticPlanComponent, SemanticPlanLimits,
                    SemanticPlanMapping, SemanticResidualPlan,
                };
                let mut required = ResidualObligationSet::new(limits());
                required.insert(item.clone()).unwrap();
                required.insert(other_item.clone()).unwrap();
                let first_component = CompiledTraceComponent::compile(
                    item.clone(),
                    template.template(),
                    &patterns,
                    &layout,
                    component_limits,
                )
                .unwrap();
                let second_component = CompiledTraceComponent::compile(
                    other_item.clone(),
                    template.template(),
                    &patterns,
                    &layout,
                    component_limits,
                )
                .unwrap();
                let first_id = first_component.id();
                let second_id = second_component.id();
                let plan = SemanticResidualPlan::build(
                    required,
                    vec![
                        SemanticPlanComponent::Trace(first_component),
                        SemanticPlanComponent::Trace(second_component),
                    ],
                    vec![layout.clone()],
                    vec![
                        SemanticPlanMapping {
                            obligation: item.id(),
                            component: first_id,
                        },
                        SemanticPlanMapping {
                            obligation: other_item.id(),
                            component: second_id,
                        },
                    ],
                    SemanticPlanLimits {
                        max_obligations: 2,
                        max_components: 2,
                        max_layouts: 1,
                        max_mappings: 2,
                        max_plan_nodes: 8,
                        identity: limits().identity,
                        layout: component_limits.layout,
                    },
                )
                .unwrap();
                assert!(matches!(
                    plan.initial_scope(scope, 1),
                    Err(ResidualScopeError::ComponentLimit)
                ));
                let mut scoped = plan.initial_scope(scope, 2).unwrap();
                assert_eq!(scoped.scope(), scope);
                assert_eq!(
                    scoped
                        .component_ids()
                        .collect::<std::collections::BTreeSet<_>>(),
                    std::collections::BTreeSet::from([first_id, second_id])
                );
                let make_attempt = || ConcreteActionAttempt {
                    scope: scope.scope,
                    activation: scope.activation,
                    revision: scope.revision,
                    invocation: scope.invocation,
                    id: ActionAttemptId::from_bytes([2; 16]),
                    cursor: SemanticEventCursor(1),
                    action: pattern.effect(),
                    bindings: layout.validate_borrowed(&values, binding_limits).unwrap(),
                };
                let attempts = BTreeMap::from([
                    (first_id, ResidualComponentAttempt::Trace(make_attempt())),
                    (second_id, ResidualComponentAttempt::Trace(make_attempt())),
                ]);
                let event = || ResidualScopeEvent {
                    id: ActionAttemptId::from_bytes([2; 16]),
                    cursor: SemanticEventCursor(1),
                    action: pattern.effect(),
                };
                assert!(matches!(
                    plan.preview_scope(&scoped, event(), &BTreeMap::new(), &NoEvidence),
                    Err(ResidualScopeError::ComponentCoverage)
                ));
                assert!(matches!(
                    plan.preview_scope(
                        &scoped,
                        ResidualScopeEvent {
                            id: ActionAttemptId::from_bytes([3; 16]),
                            ..event()
                        },
                        &attempts,
                        &NoEvidence
                    ),
                    Err(ResidualScopeError::EventMismatch(_))
                ));
                let ResidualScopeDecision::Permit(stale) = plan
                    .preview_scope(&scoped, event(), &attempts, &NoEvidence)
                    .unwrap()
                else {
                    panic!("expected scope permit")
                };
                let ResidualScopeDecision::Permit(delta) = plan
                    .preview_scope(&scoped, event(), &attempts, &NoEvidence)
                    .unwrap()
                else {
                    panic!("expected scope permit")
                };
                plan.commit_scope(&mut scoped, delta).unwrap();
                assert!(matches!(
                    plan.commit_scope(&mut scoped, stale),
                    Err(ResidualScopeError::StaleTransition)
                ));
                assert_eq!(
                    plan.finish_scope(&scoped, &[], ScopeExit::Completed, 2)
                        .unwrap(),
                    ScopeCheckResult::Accepting
                );
                assert_eq!(
                    plan.finish_scope(&scoped, &[], ScopeExit::Cancelled, 2)
                        .unwrap(),
                    ScopeCheckResult::Incomplete
                );
            }
            let make_state = |owner: &CompiledTraceComponent, obligation: &ResidualObligation| {
                owner
                    .initial_state(scope, obligation.identity().origin.revision)
                    .unwrap()
            };
            let preview = |owner: &CompiledTraceComponent, state: &_| {
                let TraceComponentDecision::Permit(delta) =
                    owner.preview(state, &attempt, &NoEvidence).unwrap()
                else {
                    panic!("expected a permitted preview")
                };
                delta
            };
            let mut first = make_state(&component, &item);
            let mut second = make_state(&other, &other_item);
            let stale = preview(&component, &first);
            let first_commit = preview(&component, &first);
            let second_commit = preview(&other, &second);
            let second_retry = preview(&other, &second);
            component.commit(&mut first, first_commit).unwrap();
            assert!(matches!(
                commit_trace_transitions(
                    vec![
                        TraceComponentTransition {
                            component: &other,
                            state: &mut second,
                            delta: second_commit
                        },
                        TraceComponentTransition {
                            component: &component,
                            state: &mut first,
                            delta: stale
                        },
                    ],
                    2
                ),
                Err(TraceBatchCommitError::InvalidTransition {
                    index: 1,
                    error: TraceComponentError::Monitor(AlternativeError::StaleTransition),
                })
            ));
            // The earlier valid transition was not partially applied.
            other.commit(&mut second, second_retry).unwrap();

            let mut first = make_state(&component, &item);
            let mut second = make_state(&other, &other_item);
            let first_delta = preview(&component, &first);
            let second_delta = preview(&other, &second);
            commit_trace_transitions(
                vec![
                    TraceComponentTransition {
                        component: &component,
                        state: &mut first,
                        delta: first_delta,
                    },
                    TraceComponentTransition {
                        component: &other,
                        state: &mut second,
                        delta: second_delta,
                    },
                ],
                2,
            )
            .unwrap();
            assert!(matches!(
                component.preview(&first, &attempt, &NoEvidence),
                Err(TraceComponentError::Monitor(AlternativeError::StaleEvent))
            ));
            assert!(matches!(
                other.preview(&second, &attempt, &NoEvidence),
                Err(TraceComponentError::Monitor(AlternativeError::StaleEvent))
            ));

            let mut limited = make_state(&component, &item);
            let rejected = preview(&component, &limited);
            let retry = preview(&component, &limited);
            assert_eq!(
                commit_trace_transitions(
                    vec![TraceComponentTransition {
                        component: &component,
                        state: &mut limited,
                        delta: rejected,
                    }],
                    0
                ),
                Err(TraceBatchCommitError::TransitionLimit)
            );
            component.commit(&mut limited, retry).unwrap();
        }
        let empty_values = BTreeMap::new();
        let wrong_attempt = ConcreteActionAttempt {
            scope: scope.scope,
            activation: scope.activation,
            revision: scope.revision,
            invocation: scope.invocation,
            id: ActionAttemptId::from_bytes([3; 16]),
            cursor: SemanticEventCursor(1),
            action: pattern.effect(),
            bindings: different_layout
                .validate_borrowed(&empty_values, binding_limits)
                .unwrap(),
        };
        assert!(matches!(
            component.preview(&state, &wrong_attempt, &NoEvidence),
            Err(TraceComponentError::LayoutMismatch)
        ));
        let missing_attempt = ConcreteActionAttempt {
            scope: scope.scope,
            activation: scope.activation,
            revision: scope.revision,
            invocation: scope.invocation,
            id: ActionAttemptId::from_bytes([4; 16]),
            cursor: SemanticEventCursor(1),
            action: pattern.effect(),
            bindings: layout
                .validate_borrowed(&empty_values, binding_limits)
                .unwrap(),
        };
        assert!(matches!(
            component.preview(&state, &missing_attempt, &NoEvidence),
            Err(TraceComponentError::MissingBindings { count: 1 })
        ));
        let TraceComponentDecision::Permit(delta) =
            component.preview(&state, &attempt, &NoEvidence).unwrap()
        else {
            panic!("expected valid transition")
        };
        component.commit(&mut state, delta).unwrap();
        assert!(matches!(
            component.preview(&state, &attempt, &NoEvidence),
            Err(TraceComponentError::Monitor(
                crate::residual::monitor::AlternativeError::StaleEvent
            ))
        ));
        assert_eq!(
            component
                .finish_scope(&state, &[], ScopeExit::Completed)
                .unwrap(),
            ScopeCheckResult::Accepting
        );

        let mut guard_effect = pattern.effect().clone();
        let EffectTargetInfo::Action { name, .. } = &mut guard_effect.target else {
            panic!("expected action target");
        };
        *name = "approve".into();
        let guard = ActionPattern::new(guard_effect, pattern_limits).unwrap();
        let temporal = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Before {
                guard: guard.clone(),
                target: pattern.clone(),
            }]],
            monitor_limits,
        )
        .unwrap();
        let temporal_item = ResidualObligation::new(
            identity_for_rule(&base, ResidualRuleKind::TraceContract),
            ResidualKind::TraceContract {
                monitor: temporal.template_id(),
            },
            layout.id(),
            limits().identity,
        )
        .unwrap();
        let temporal_component = CompiledTraceComponent::compile(
            temporal_item.clone(),
            temporal.template(),
            &BTreeMap::from([(guard.id(), guard.clone()), (pattern.id(), pattern.clone())]),
            &layout,
            component_limits,
        )
        .unwrap();
        let mut temporal_state = temporal_component
            .initial_state(scope, temporal_item.identity().origin.revision)
            .unwrap();
        struct Evidence(bool);
        impl ConcretePriorEvidence for Evidence {
            fn query(&self, query: EvidenceQuery<'_, '_>) -> EvidenceStatus {
                if !self.0 {
                    return EvidenceStatus::Pending;
                }
                EvidenceStatus::Accepted(crate::residual::monitor::AcceptedPriorEvidence {
                    scope: query.attempt.scope,
                    activation: query.attempt.activation,
                    revision: query.attempt.revision,
                    invocation: query.attempt.invocation,
                    guard_pattern: query.guard.id(),
                    target_pattern: query.target.id(),
                    guard_attempt: ActionAttemptId::from_bytes([4; 16]),
                    completed_at: SemanticEventCursor(0),
                })
            }
        }
        {
            use crate::residual::plan::{
                ResidualComponentAttempt, ResidualScopeDecision, ResidualScopeError,
                ResidualScopeEvent, SemanticPlanComponent, SemanticPlanLimits, SemanticPlanMapping,
                SemanticResidualPlan,
            };
            struct Denied;
            impl ConcretePriorEvidence for Denied {
                fn query(&self, _: EvidenceQuery<'_, '_>) -> EvidenceStatus {
                    EvidenceStatus::Denied
                }
            }
            let mut identity = temporal_item.identity().clone();
            identity.clause = ClauseId::from_canonical_fingerprint(fp(79));
            let guarded_item = ResidualObligation::new(
                identity,
                temporal_item.kind().clone(),
                layout.id(),
                limits().identity,
            )
            .unwrap();
            let allow = CompiledTraceComponent::compile(
                item.clone(),
                template.template(),
                &patterns,
                &layout,
                component_limits,
            )
            .unwrap();
            let guarded = CompiledTraceComponent::compile(
                guarded_item.clone(),
                temporal.template(),
                &BTreeMap::from([(guard.id(), guard.clone()), (pattern.id(), pattern.clone())]),
                &layout,
                component_limits,
            )
            .unwrap();
            let (allow_id, guarded_id) = (allow.id(), guarded.id());
            let mut required = ResidualObligationSet::new(limits());
            required.insert(item.clone()).unwrap();
            required.insert(guarded_item.clone()).unwrap();
            let plan = SemanticResidualPlan::build(
                required,
                vec![
                    SemanticPlanComponent::Trace(allow),
                    SemanticPlanComponent::Trace(guarded),
                ],
                vec![layout.clone()],
                vec![
                    SemanticPlanMapping {
                        obligation: item.id(),
                        component: allow_id,
                    },
                    SemanticPlanMapping {
                        obligation: guarded_item.id(),
                        component: guarded_id,
                    },
                ],
                SemanticPlanLimits {
                    max_obligations: 2,
                    max_components: 2,
                    max_layouts: 1,
                    max_mappings: 2,
                    max_plan_nodes: 8,
                    identity: limits().identity,
                    layout: component_limits.layout,
                },
            )
            .unwrap();
            let mut scoped = plan.initial_scope(scope, 2).unwrap();
            let make_attempt = || ConcreteActionAttempt {
                scope: scope.scope,
                activation: scope.activation,
                revision: scope.revision,
                invocation: scope.invocation,
                id: ActionAttemptId::from_bytes([2; 16]),
                cursor: SemanticEventCursor(1),
                action: pattern.effect(),
                bindings: layout.validate_borrowed(&values, binding_limits).unwrap(),
            };
            let attempts = BTreeMap::from([
                (allow_id, ResidualComponentAttempt::Trace(make_attempt())),
                (guarded_id, ResidualComponentAttempt::Trace(make_attempt())),
            ]);
            let event = || ResidualScopeEvent {
                id: ActionAttemptId::from_bytes([2; 16]),
                cursor: SemanticEventCursor(1),
                action: pattern.effect(),
            };
            assert!(matches!(
                plan.preview_scope(&scoped, event(), &attempts, &Denied)
                    .unwrap(),
                ResidualScopeDecision::Reject
            ));
            let ResidualScopeDecision::NeedsEvidence(pending) = plan
                .preview_scope(&scoped, event(), &attempts, &Evidence(false))
                .unwrap()
            else {
                panic!("pending evidence must not permit the scope")
            };
            assert_eq!(pending.len(), 1);
            assert_eq!(pending[0].component, guarded_id);
            assert!(matches!(
                plan.finish_scope(&scoped, &pending, ScopeExit::Completed, 0),
                Err(ResidualScopeError::PendingLimit)
            ));
            assert_eq!(
                plan.finish_scope(&scoped, &pending, ScopeExit::Completed, 2)
                    .unwrap(),
                ScopeCheckResult::PendingEvidence
            );
            assert_eq!(
                plan.finish_scope(&scoped, &pending, ScopeExit::Cancelled, 2)
                    .unwrap(),
                ScopeCheckResult::Incomplete
            );
            let ResidualScopeDecision::Permit(delta) = plan
                .preview_scope(&scoped, event(), &attempts, &Evidence(true))
                .unwrap()
            else {
                panic!("accepted evidence should permit")
            };
            plan.commit_scope(&mut scoped, delta).unwrap();
            // A checkpointed scope restores to an equivalent state through the
            // same plan; tampered, foreign or incomplete data is rejected.
            let data = scoped.to_data();
            let restored = plan.restore_scope(scope, &data, 2).unwrap();
            assert_eq!(restored.to_data(), data);
            assert_eq!(restored.scope(), scope);
            let mut foreign = data.clone();
            foreign.plan = crate::residual::identity::PlanHash::from_canonical_fingerprint(
                crate::CodeFingerprint::from_bytes([9; 32]),
            );
            assert!(matches!(
                plan.restore_scope(scope, &foreign, 2),
                Err(ResidualScopeError::PlanMismatch)
            ));
            let mut missing = data.clone();
            missing.components.pop();
            assert!(matches!(
                plan.restore_scope(scope, &missing, 2),
                Err(ResidualScopeError::ComponentCoverage)
            ));
            let mut emptied = data.clone();
            emptied.components[0].trace.as_mut().unwrap().active.clear();
            assert!(plan.restore_scope(scope, &emptied, 2).is_err());
            let mut unknown_alternative = data.clone();
            unknown_alternative.components[0]
                .trace
                .as_mut()
                .unwrap()
                .active[0]
                .alternative = 99;
            assert!(plan.restore_scope(scope, &unknown_alternative, 2).is_err());
            let mut rejected = data.clone();
            for branch in &mut rejected.components[1].trace.as_mut().unwrap().active {
                for component in &mut branch.components {
                    component.rejected = true;
                }
            }
            if rejected != data {
                assert!(plan.restore_scope(scope, &rejected, 2).is_err());
            }
            let other_scope = AlternativeScope {
                invocation: ScopeInvocationId::from_bytes([77; 16]),
                ..scope
            };
            assert_eq!(
                plan.restore_scope(other_scope, &data, 2).unwrap().scope(),
                other_scope
            );
            assert!(matches!(
                plan.finish_scope(&scoped, &pending, ScopeExit::Completed, 2),
                Err(ResidualScopeError::Component {
                    error: TraceComponentError::Monitor(
                        crate::residual::monitor::AlternativeError::StaleTransition
                    ),
                    ..
                })
            ));
        }
        let TraceComponentDecision::NeedsEvidence(pending) = temporal_component
            .preview(&temporal_state, &attempt, &Evidence(false))
            .unwrap()
        else {
            panic!("pending evidence must not permit")
        };
        assert_eq!(
            temporal_component
                .finish_scope(
                    &temporal_state,
                    std::slice::from_ref(&pending),
                    ScopeExit::Completed
                )
                .unwrap(),
            ScopeCheckResult::PendingEvidence
        );
        assert!(matches!(
            component.finish_scope(&state, std::slice::from_ref(&pending), ScopeExit::Completed),
            Err(TraceComponentError::ComponentMismatch)
        ));
        let TraceComponentDecision::Permit(delta) = temporal_component
            .preview(&temporal_state, &attempt, &Evidence(true))
            .unwrap()
        else {
            panic!("verified evidence should permit the monitor transition")
        };
        assert_eq!(
            component.commit(&mut state, delta),
            Err(TraceComponentError::ComponentMismatch)
        );
        let TraceComponentDecision::Permit(delta) = temporal_component
            .preview(&temporal_state, &attempt, &Evidence(true))
            .unwrap()
        else {
            panic!("verified evidence should permit the monitor transition")
        };
        temporal_component
            .commit(&mut temporal_state, delta)
            .unwrap();
        assert!(matches!(
            temporal_component.finish_scope(&temporal_state, &[pending], ScopeExit::Completed),
            Err(TraceComponentError::Monitor(
                crate::residual::monitor::AlternativeError::StaleTransition
            ))
        ));
        assert_eq!(
            temporal_component
                .finish_scope(&temporal_state, &[], ScopeExit::Completed)
                .unwrap(),
            ScopeCheckResult::Accepting
        );
    }

    #[test]
    fn obligation_binds_exact_predicate_and_checked_layout() {
        let descriptor = BindingDescriptor {
            root: BindingRoot::Parameter { position: 0 },
            fields: vec![],
            value_type: ValueType::Text,
        };
        let value = descriptor.id(2).unwrap();
        let node = PredicateNode::Equal {
            left: value,
            right: value,
        };
        let predicate_id = PredicateId::from_node(&node);
        let graph = PredicateGraph {
            root: predicate_id,
            nodes: BTreeMap::from([(predicate_id, node)]),
        };
        let mut schema = PredicateSchema::default();
        schema.bindings.insert(value, ValueType::Text);
        let predicate = graph
            .bind(
                &schema,
                PredicateLimits {
                    max_nodes: 4,
                    max_edges: 4,
                    max_depth: 4,
                    max_evidence_arguments: 2,
                },
            )
            .unwrap();
        let layout_limits = BindingLayoutLimits {
            max_bindings: 2,
            max_projections_per_binding: 2,
            max_total_projections: 2,
        };
        let layout =
            BindingLayout::new(BTreeMap::from([(value, descriptor)]), layout_limits).unwrap();
        let other_descriptor = BindingDescriptor {
            root: BindingRoot::Parameter { position: 0 },
            fields: vec![],
            value_type: ValueType::Bytes,
        };
        let other_value = other_descriptor.id(2).unwrap();
        let other_layout = BindingLayout::new(
            BTreeMap::from([(other_value, other_descriptor)]),
            layout_limits,
        )
        .unwrap();
        let effect_owner = DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(fp(21)),
            vec!["app".into()],
            vec!["Effect".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        let pattern_limits = ActionPatternLimits {
            max_selectors: 4,
            max_identity_segments: 8,
            max_identity_bytes: 64,
            max_literal_bytes: 64,
        };
        let pattern = ActionPattern::new(
            CanonicalEffect {
                target: EffectTargetInfo::Action {
                    effect: effect_owner.clone(),
                    name: "request".into(),
                },
                arguments: vec![],
            },
            pattern_limits,
        )
        .unwrap();
        let other_pattern = ActionPattern::new(
            CanonicalEffect {
                target: EffectTargetInfo::Action {
                    effect: effect_owner,
                    name: "other".into(),
                },
                arguments: vec![],
            },
            pattern_limits,
        )
        .unwrap();
        let base = obligation(7, 10);
        let kind = ResidualKind::ActionPattern {
            pattern: pattern.id(),
            predicate: predicate_id,
        };
        let item = ResidualObligation::new(
            base.identity().clone(),
            kind,
            layout.id(),
            limits().identity,
        )
        .unwrap();
        assert_eq!(item.validate_predicate_binding(&predicate, &layout), Ok(()));
        assert_eq!(
            item.validate_action_pattern_binding(&pattern, &predicate, &layout),
            Ok(())
        );
        let eval_limits = PredicateEvaluationLimits {
            max_nodes: 4,
            max_bindings: 2,
            max_work: 8,
            max_value_bytes: 16,
            max_comparison_bytes: 32,
        };
        let values = BTreeMap::from([(value, BorrowedValue::Text("draft"))]);
        let empty_values = BTreeMap::new();
        let input = |action, bindings| ActionPredicateInput {
            scope: item.identity().scope,
            activation: item.identity().activation,
            source_revision: item.identity().origin.revision,
            pattern: &pattern,
            predicate: &predicate,
            layout: &layout,
            action,
            bindings,
            pattern_limits,
            evaluation_limits: eval_limits,
        };
        assert_eq!(
            item.evaluate_action_pattern(input(pattern.effect(), &values)),
            Ok(ActionPredicateDecision::Applicable(
                PredicateDecision::Satisfied
            ))
        );
        let component_limits = ActionComponentLimits {
            identity: limits().identity,
            pattern: pattern_limits,
            predicate: PredicateLimits {
                max_nodes: 4,
                max_edges: 4,
                max_depth: 4,
                max_evidence_arguments: 2,
            },
            layout: layout_limits,
            evaluation: eval_limits,
        };
        let component = CompiledActionPredicate::compile(
            item.clone(),
            pattern.clone(),
            predicate.clone(),
            layout.clone(),
            component_limits,
        )
        .unwrap();
        {
            use crate::residual::monitor::{
                ActionAttemptId, AlternativeLimits, AlternativeRule, AlternativeScope,
                CompiledTraceAlternatives, CompiledTraceComponent, ConcreteActionAttempt,
                ConcreteEvidenceLimits, ConcretePriorEvidence, EvidenceQuery, EvidenceStatus,
                ScopeCheckResult, ScopeExit, ScopeInvocationId, SemanticEventCursor,
                TraceComponentLimits,
            };
            use crate::residual::plan::{
                ResidualActionAttempt, ResidualComponentAttempt, ResidualScopeDecision,
                ResidualScopeError, ResidualScopeEvent, SemanticPlanComponent, SemanticPlanError,
                SemanticPlanLimits, SemanticPlanMapping, SemanticResidualPlan,
            };

            let monitor_limits = AlternativeLimits {
                evidence: ConcreteEvidenceLimits {
                    pattern: pattern_limits,
                    max_bindings: 2,
                    max_value_bytes: 16,
                },
                max_alternatives: 2,
                max_clauses: 4,
                max_patterns: 4,
                max_checks_per_event: 8,
                max_product_work_per_event: 16,
            };
            let template = CompiledTraceAlternatives::compile(
                &[vec![AlternativeRule::Allow(pattern.clone())]],
                monitor_limits,
            )
            .unwrap();
            let trace = ResidualObligation::new(
                identity_for_rule(&base, ResidualRuleKind::TraceContract),
                ResidualKind::TraceContract {
                    monitor: template.template_id(),
                },
                layout.id(),
                limits().identity,
            )
            .unwrap();
            let make_components = || {
                vec![
                    SemanticPlanComponent::Action(
                        CompiledActionPredicate::compile(
                            item.clone(),
                            pattern.clone(),
                            predicate.clone(),
                            layout.clone(),
                            component_limits,
                        )
                        .unwrap(),
                    ),
                    SemanticPlanComponent::Trace(
                        CompiledTraceComponent::compile(
                            trace.clone(),
                            template.template(),
                            &BTreeMap::from([(pattern.id(), pattern.clone())]),
                            &layout,
                            TraceComponentLimits {
                                identity: limits().identity,
                                layout: layout_limits,
                                monitor: monitor_limits,
                            },
                        )
                        .unwrap(),
                    ),
                ]
            };
            let make_obligations = || {
                let mut obligations = ResidualObligationSet::new(limits());
                obligations.insert(item.clone()).unwrap();
                obligations.insert(trace.clone()).unwrap();
                obligations
            };
            let components = make_components();
            let mappings = vec![
                SemanticPlanMapping {
                    obligation: item.id(),
                    component: components[0].id(),
                },
                SemanticPlanMapping {
                    obligation: trace.id(),
                    component: components[1].id(),
                },
            ];
            let plan_limits = SemanticPlanLimits {
                max_obligations: 4,
                max_components: 4,
                max_layouts: 4,
                max_mappings: 4,
                max_plan_nodes: 8,
                identity: limits().identity,
                layout: layout_limits,
            };
            let plan = SemanticResidualPlan::build(
                make_obligations(),
                components,
                vec![layout.clone()],
                mappings.clone(),
                plan_limits,
            )
            .unwrap();
            assert_eq!(plan.obligations().len(), 2);
            assert!(plan.component_for(item.id()).is_some());
            assert!(plan.component_for(trace.id()).is_some());
            {
                use crate::residual::identity::{
                    GenericArgumentIdentity, GenericParameterId, ResidualObligationRef,
                };
                use crate::residual::plan::{
                    BoundResidualPlan, BoundResidualPlanData, BoundResidualPlanError,
                };
                let definition = crate::DefinitionKey::new(
                    crate::PackageKey::from_name_and_version("sample", "1.0.0").unwrap(),
                    vec!["sample".into()],
                    vec!["run".into()],
                    crate::DefinitionKind::Flow,
                )
                .unwrap();
                let revision = crate::RevisionInfo::new(crate::RevisionInputs {
                    definition: crate::CallableId::declaration(definition).unwrap(),
                    content: fp(71),
                    environment: fp(72),
                    contract: fp(73),
                    compilation: fp(74),
                    parent: None,
                });
                let reference = |id| ResidualObligationRef::new(id, Default::default()).unwrap();
                let expected = [reference(item.id()), reference(trace.id())]
                    .into_iter()
                    .collect();
                let bound = BoundResidualPlan::bind(&plan, &revision, &expected, 2).unwrap();
                let data: BoundResidualPlanData =
                    serde_json::from_str(&serde_json::to_string(bound.data()).unwrap()).unwrap();
                let mut newer_inputs = revision.inputs().clone();
                newer_inputs.content = fp(82);
                let newer = crate::RevisionInfo::new(newer_inputs);
                let newer_bound = BoundResidualPlan::bind(&plan, &newer, &expected, 2).unwrap();
                assert_eq!(newer_bound.data().plan, bound.data().plan);
                assert_ne!(newer_bound.data().fingerprint, bound.data().fingerprint);
                assert_eq!(
                    BoundResidualPlan::from_data(data.clone(), &plan, &newer, &expected, 2)
                        .unwrap_err(),
                    BoundResidualPlanError::RevisionMismatch
                );
                assert_eq!(
                    BoundResidualPlan::from_data(data.clone(), &plan, &revision, &expected, 2)
                        .unwrap(),
                    bound
                );
                assert!(matches!(
                    BoundResidualPlan::bind(&plan, &revision, &expected, 1),
                    Err(BoundResidualPlanError::ObligationLimit)
                ));
                let incomplete = [reference(item.id())].into_iter().collect();
                assert!(matches!(
                    BoundResidualPlan::bind(&plan, &revision, &incomplete, 2),
                    Err(BoundResidualPlanError::UnexpectedObligation(_))
                ));
                let extra = [
                    reference(item.id()),
                    reference(trace.id()),
                    reference(ObligationId::from_canonical_fingerprint(fp(75))),
                ]
                .into_iter()
                .collect();
                assert!(matches!(
                    BoundResidualPlan::bind(&plan, &revision, &extra, 3),
                    Err(BoundResidualPlanError::MissingObligation(_))
                ));
                let parameter = GenericParameterId::from_canonical_fingerprint(fp(83));
                let substituted = ResidualObligationRef::new(
                    item.id(),
                    BTreeMap::from([(parameter, GenericArgumentIdentity::Type(fp(84)))]),
                )
                .unwrap();
                let substituted_other = ResidualObligationRef::new(
                    item.id(),
                    BTreeMap::from([(parameter, GenericArgumentIdentity::Type(fp(85)))]),
                )
                .unwrap();
                let generic = [substituted, substituted_other, reference(trace.id())]
                    .into_iter()
                    .collect();
                assert_eq!(
                    BoundResidualPlan::bind(&plan, &revision, &generic, 3).unwrap_err(),
                    BoundResidualPlanError::UnsupportedSubstitution(item.id())
                );
                assert_eq!(
                    BoundResidualPlan::from_data(data.clone(), &plan, &revision, &generic, 3)
                        .unwrap_err(),
                    BoundResidualPlanError::UnsupportedSubstitution(item.id())
                );
                let mut changed = data.clone();
                changed.schema_version += 1;
                assert!(matches!(
                    BoundResidualPlan::from_data(changed, &plan, &revision, &expected, 2),
                    Err(BoundResidualPlanError::UnsupportedSchema(_))
                ));
                let mut changed = data.clone();
                changed.plan =
                    crate::residual::identity::PlanHash::from_canonical_fingerprint(fp(76));
                assert_eq!(
                    BoundResidualPlan::from_data(changed, &plan, &revision, &expected, 2)
                        .unwrap_err(),
                    BoundResidualPlanError::PlanMismatch
                );
                let mut changed = data.clone();
                changed.revision = crate::RevisionKey::from_canonical_fingerprint(fp(77));
                assert_eq!(
                    BoundResidualPlan::from_data(changed, &plan, &revision, &expected, 2)
                        .unwrap_err(),
                    BoundResidualPlanError::RevisionMismatch
                );
                let mut changed = data.clone();
                changed.contract = fp(78);
                assert_eq!(
                    BoundResidualPlan::from_data(changed, &plan, &revision, &expected, 2)
                        .unwrap_err(),
                    BoundResidualPlanError::ContractMismatch
                );
                let mut changed = data.clone();
                changed.environment = fp(79);
                assert_eq!(
                    BoundResidualPlan::from_data(changed, &plan, &revision, &expected, 2)
                        .unwrap_err(),
                    BoundResidualPlanError::EnvironmentMismatch
                );
                let mut changed = data.clone();
                changed.compilation = fp(80);
                assert_eq!(
                    BoundResidualPlan::from_data(changed, &plan, &revision, &expected, 2)
                        .unwrap_err(),
                    BoundResidualPlanError::CompilationMismatch
                );
                let mut changed = data;
                changed.fingerprint = fp(81);
                assert_eq!(
                    BoundResidualPlan::from_data(changed, &plan, &revision, &expected, 2)
                        .unwrap_err(),
                    BoundResidualPlanError::FingerprintMismatch
                );
            }
            struct NoEvidence;
            impl ConcretePriorEvidence for NoEvidence {
                fn query(&self, _: EvidenceQuery<'_, '_>) -> EvidenceStatus {
                    EvidenceStatus::Unavailable
                }
            }
            let scope = AlternativeScope {
                scope: item.identity().scope,
                activation: item.identity().activation,
                revision: crate::RevisionKey::from_canonical_fingerprint(fp(99)),
                invocation: ScopeInvocationId::from_bytes([1; 16]),
            };
            let mut scope_state = plan.initial_scope(scope, 2).unwrap();
            let event = || ResidualScopeEvent {
                id: ActionAttemptId::from_bytes([3; 16]),
                cursor: SemanticEventCursor(1),
                action: pattern.effect(),
            };
            let attempts = |action_bindings| {
                BTreeMap::from([
                    (
                        mappings[0].component,
                        ResidualComponentAttempt::Action(ResidualActionAttempt {
                            id: ActionAttemptId::from_bytes([3; 16]),
                            cursor: SemanticEventCursor(1),
                            revision: scope.revision,
                            invocation: scope.invocation,
                            input: ActionComponentInput {
                                scope: scope.scope,
                                activation: scope.activation,
                                source_revision: item.identity().origin.revision,
                                action: pattern.effect(),
                                bindings: action_bindings,
                            },
                        }),
                    ),
                    (
                        mappings[1].component,
                        ResidualComponentAttempt::Trace(ConcreteActionAttempt {
                            scope: scope.scope,
                            activation: scope.activation,
                            revision: scope.revision,
                            invocation: scope.invocation,
                            id: ActionAttemptId::from_bytes([3; 16]),
                            cursor: SemanticEventCursor(1),
                            action: pattern.effect(),
                            bindings: layout
                                .validate_borrowed(
                                    &values,
                                    crate::residual::binding::BorrowedBindingLimits {
                                        max_bindings: 2,
                                        max_value_bytes: 16,
                                        max_total_value_bytes: 32,
                                    },
                                )
                                .unwrap(),
                        }),
                    ),
                ])
            };
            let mut wrong_event = attempts(&values);
            let Some(ResidualComponentAttempt::Action(action)) =
                wrong_event.get_mut(&mappings[0].component)
            else {
                panic!("expected action")
            };
            action.id = ActionAttemptId::from_bytes([4; 16]);
            assert!(
                matches!(plan.preview_scope(&scope_state, event(), &wrong_event, &NoEvidence),
                Err(ResidualScopeError::EventMismatch(id)) if id == mappings[0].component)
            );
            assert!(matches!(plan.preview_scope(
                &scope_state, event(), &attempts(&empty_values), &NoEvidence,
            ), Err(ResidualScopeError::MissingActionBinding(id)) if id == mappings[0].component));
            let ResidualScopeDecision::Permit(delta) = plan
                .preview_scope(&scope_state, event(), &attempts(&values), &NoEvidence)
                .unwrap()
            else {
                panic!("matched predicate should permit")
            };
            plan.commit_scope(&mut scope_state, delta).unwrap();
            assert_eq!(
                plan.finish_scope(&scope_state, &[], ScopeExit::Completed, 2)
                    .unwrap(),
                ScopeCheckResult::Accepting
            );
            {
                let mut required = ResidualObligationSet::new(limits());
                required.insert(item.clone()).unwrap();
                let action = make_components().remove(0);
                let action_id = action.id();
                let action_only = SemanticResidualPlan::build(
                    required,
                    vec![action],
                    vec![layout.clone()],
                    vec![SemanticPlanMapping {
                        obligation: item.id(),
                        component: action_id,
                    }],
                    plan_limits,
                )
                .unwrap();
                let mut state = action_only.initial_scope(scope, 1).unwrap();
                let attempts = BTreeMap::from([(
                    action_id,
                    ResidualComponentAttempt::Action(ResidualActionAttempt {
                        id: ActionAttemptId::from_bytes([3; 16]),
                        cursor: SemanticEventCursor(1),
                        revision: scope.revision,
                        invocation: scope.invocation,
                        input: ActionComponentInput {
                            scope: scope.scope,
                            activation: scope.activation,
                            source_revision: item.identity().origin.revision,
                            action: pattern.effect(),
                            bindings: &values,
                        },
                    }),
                )]);
                let ResidualScopeDecision::Permit(first) = action_only
                    .preview_scope(&state, event(), &attempts, &NoEvidence)
                    .unwrap()
                else {
                    panic!("expected permit")
                };
                let ResidualScopeDecision::Permit(stale) = action_only
                    .preview_scope(&state, event(), &attempts, &NoEvidence)
                    .unwrap()
                else {
                    panic!("expected permit")
                };
                action_only.commit_scope(&mut state, first).unwrap();
                assert!(matches!(
                    action_only.commit_scope(&mut state, stale),
                    Err(ResidualScopeError::StaleTransition)
                ));
                assert!(matches!(
                    action_only.preview_scope(&state, event(), &attempts, &NoEvidence),
                    Err(ResidualScopeError::StaleEvent)
                ));
                assert_eq!(
                    action_only
                        .finish_scope(&state, &[], ScopeExit::Failed, 1)
                        .unwrap(),
                    ScopeCheckResult::Incomplete
                );
            }
            {
                let second_descriptor = BindingDescriptor {
                    root: BindingRoot::Parameter { position: 1 },
                    fields: vec![],
                    value_type: ValueType::Text,
                };
                let second_value = second_descriptor.id(2).unwrap();
                let two_layout = BindingLayout::new(
                    BTreeMap::from([
                        (value, layout.descriptor(value).unwrap().clone()),
                        (second_value, second_descriptor),
                    ]),
                    layout_limits,
                )
                .unwrap();
                let node = PredicateNode::Equal {
                    left: value,
                    right: second_value,
                };
                let predicate_id = PredicateId::from_node(&node);
                let graph = PredicateGraph {
                    root: predicate_id,
                    nodes: BTreeMap::from([(predicate_id, node)]),
                };
                let mut schema = PredicateSchema::default();
                schema.bindings.insert(value, ValueType::Text);
                schema.bindings.insert(second_value, ValueType::Text);
                let comparison = graph.bind(&schema, component_limits.predicate).unwrap();
                let action_item = ResidualObligation::new(
                    item.identity().clone(),
                    ResidualKind::ActionPattern {
                        pattern: pattern.id(),
                        predicate: predicate_id,
                    },
                    two_layout.id(),
                    limits().identity,
                )
                .unwrap();
                let action = CompiledActionPredicate::compile(
                    action_item.clone(),
                    pattern.clone(),
                    comparison,
                    two_layout.clone(),
                    component_limits,
                )
                .unwrap();
                let trace_component = CompiledTraceComponent::compile(
                    trace.clone(),
                    template.template(),
                    &BTreeMap::from([(pattern.id(), pattern.clone())]),
                    &layout,
                    TraceComponentLimits {
                        identity: limits().identity,
                        layout: layout_limits,
                        monitor: monitor_limits,
                    },
                )
                .unwrap();
                let (action_id, trace_id) = (action.id(), trace_component.id());
                let mut required = ResidualObligationSet::new(limits());
                required.insert(action_item.clone()).unwrap();
                required.insert(trace.clone()).unwrap();
                let mixed = SemanticResidualPlan::build(
                    required,
                    vec![
                        SemanticPlanComponent::Action(action),
                        SemanticPlanComponent::Trace(trace_component),
                    ],
                    vec![layout.clone(), two_layout],
                    vec![
                        SemanticPlanMapping {
                            obligation: action_item.id(),
                            component: action_id,
                        },
                        SemanticPlanMapping {
                            obligation: trace.id(),
                            component: trace_id,
                        },
                    ],
                    SemanticPlanLimits {
                        max_plan_nodes: 11,
                        ..plan_limits
                    },
                )
                .unwrap();
                let mut state = mixed.initial_scope(scope, 2).unwrap();
                let unequal = BTreeMap::from([
                    (value, BorrowedValue::Text("draft")),
                    (second_value, BorrowedValue::Text("other")),
                ]);
                let equal = BTreeMap::from([
                    (value, BorrowedValue::Text("draft")),
                    (second_value, BorrowedValue::Text("draft")),
                ]);
                let make_attempts = |action_bindings| {
                    BTreeMap::from([
                        (
                            action_id,
                            ResidualComponentAttempt::Action(ResidualActionAttempt {
                                id: ActionAttemptId::from_bytes([3; 16]),
                                cursor: SemanticEventCursor(1),
                                revision: scope.revision,
                                invocation: scope.invocation,
                                input: ActionComponentInput {
                                    scope: scope.scope,
                                    activation: scope.activation,
                                    source_revision: item.identity().origin.revision,
                                    action: pattern.effect(),
                                    bindings: action_bindings,
                                },
                            }),
                        ),
                        (
                            trace_id,
                            ResidualComponentAttempt::Trace(ConcreteActionAttempt {
                                scope: scope.scope,
                                activation: scope.activation,
                                revision: scope.revision,
                                invocation: scope.invocation,
                                id: ActionAttemptId::from_bytes([3; 16]),
                                cursor: SemanticEventCursor(1),
                                action: pattern.effect(),
                                bindings: layout
                                    .validate_borrowed(
                                        &values,
                                        crate::residual::binding::BorrowedBindingLimits {
                                            max_bindings: 2,
                                            max_value_bytes: 16,
                                            max_total_value_bytes: 32,
                                        },
                                    )
                                    .unwrap(),
                            }),
                        ),
                    ])
                };
                assert!(matches!(
                    mixed
                        .preview_scope(&state, event(), &make_attempts(&unequal), &NoEvidence)
                        .unwrap(),
                    ResidualScopeDecision::Reject
                ));
                let ResidualScopeDecision::Permit(delta) = mixed
                    .preview_scope(&state, event(), &make_attempts(&equal), &NoEvidence)
                    .unwrap()
                else {
                    panic!("equal values should permit both components")
                };
                mixed.commit_scope(&mut state, delta).unwrap();
            }
            let mut reversed_components = make_components();
            reversed_components.reverse();
            let mut reversed_mappings = mappings.clone();
            reversed_mappings.reverse();
            let reordered = SemanticResidualPlan::build(
                make_obligations(),
                reversed_components,
                vec![layout.clone()],
                reversed_mappings,
                plan_limits,
            )
            .unwrap();
            assert_eq!(plan.hash(), reordered.hash());
            assert!(
                SemanticResidualPlan::from_parts(
                    plan.hash(),
                    make_obligations(),
                    make_components(),
                    vec![layout.clone()],
                    mappings.clone(),
                    plan_limits,
                )
                .is_ok()
            );
            assert!(matches!(
                SemanticResidualPlan::build(
                    make_obligations(),
                    make_components(),
                    vec![layout.clone()],
                    mappings.clone(),
                    SemanticPlanLimits {
                        max_components: 1,
                        ..plan_limits
                    },
                ),
                Err(SemanticPlanError::ComponentLimit)
            ));
            assert!(matches!(
                SemanticResidualPlan::build(
                    make_obligations(),
                    make_components(),
                    vec![layout.clone()],
                    mappings.clone(),
                    SemanticPlanLimits {
                        max_plan_nodes: 7,
                        ..plan_limits
                    },
                ),
                Err(SemanticPlanError::PlanNodeLimit)
            ));
            assert!(matches!(
                SemanticResidualPlan::build(
                    make_obligations(),
                    make_components(),
                    vec![layout.clone(), other_layout.clone()],
                    mappings.clone(),
                    SemanticPlanLimits {
                        max_plan_nodes: 9,
                        ..plan_limits
                    },
                ),
                Err(SemanticPlanError::PlanNodeLimit)
            ));
            let larger_budget = SemanticResidualPlan::build(
                make_obligations(),
                make_components(),
                vec![layout.clone()],
                mappings.clone(),
                SemanticPlanLimits {
                    max_plan_nodes: 9,
                    ..plan_limits
                },
            )
            .unwrap();
            assert_ne!(plan.hash(), larger_budget.hash());
            assert!(matches!(
                SemanticResidualPlan::from_parts(
                    crate::residual::identity::PlanHash::from_canonical_fingerprint(fp(99)),
                    make_obligations(),
                    make_components(),
                    vec![layout.clone()],
                    mappings.clone(),
                    plan_limits,
                ),
                Err(SemanticPlanError::HashMismatch)
            ));
            assert!(matches!(
                SemanticResidualPlan::build(
                    make_obligations(), make_components(), vec![layout.clone()],
                    vec![mappings[0]], plan_limits,
                ),
                Err(SemanticPlanError::MissingObligation(id)) if id == trace.id()
            ));
            assert!(matches!(
                SemanticResidualPlan::build(
                    make_obligations(), make_components(), vec![layout.clone()],
                    vec![mappings[0], mappings[0]], plan_limits,
                ),
                Err(SemanticPlanError::DuplicateObligation(id)) if id == item.id()
            ));
            assert!(matches!(
                SemanticResidualPlan::build(
                    make_obligations(), make_components(), vec![layout.clone()],
                    vec![SemanticPlanMapping { obligation: item.id(), component: mappings[1].component }],
                    plan_limits,
                ),
                Err(SemanticPlanError::IncompatibleComponent(id)) if id == item.id()
            ));
            assert!(matches!(
                SemanticResidualPlan::build(
                    make_obligations(), make_components(), vec![], mappings, plan_limits,
                ),
                Err(SemanticPlanError::MissingLayout(id)) if id == layout.id()
            ));
        }
        let mut sibling_identity = item.identity().clone();
        sibling_identity.clause = ClauseId::from_canonical_fingerprint(fp(44));
        sibling_identity.origin.byte_start = 9;
        sibling_identity.origin.byte_end = 12;
        let sibling = ResidualObligation::new(
            sibling_identity,
            item.kind().clone(),
            item.bindings(),
            limits().identity,
        )
        .unwrap();
        let mut required = ResidualObligationSet::new(limits());
        required.insert(item.clone()).unwrap();
        let components = BTreeMap::from([(component.id(), &component)]);
        let mappings = [ActionCoverageEntry {
            obligation: item.id(),
            component: component.id(),
        }];
        let coverage_limits = ActionCoverageLimits {
            max_obligations: 4,
            max_components: 2,
            max_mappings: 4,
        };
        let coverage =
            verify_action_component_coverage(&required, &components, &mappings, coverage_limits)
                .unwrap();
        assert_eq!(coverage.len(), 1);
        assert!(!coverage.is_empty());
        assert_eq!(coverage.get(item.id()).unwrap().id(), component.id());
        assert!(
            matches!(verify_action_component_coverage(&required, &components, &[], coverage_limits), Err(ActionCoverageError::MissingObligation(id)) if id == item.id())
        );
        assert!(
            matches!(verify_action_component_coverage(&required, &components, &[mappings[0], mappings[0]], coverage_limits), Err(ActionCoverageError::DuplicateObligation(id)) if id == item.id())
        );
        let missing_component = MonitorTemplateId::from_canonical_fingerprint(fp(45));
        assert!(
            matches!(verify_action_component_coverage(&required, &components, &[ActionCoverageEntry { obligation: item.id(), component: missing_component }], coverage_limits), Err(ActionCoverageError::MissingComponent(id)) if id == missing_component)
        );
        assert!(
            matches!(verify_action_component_coverage(&required, &BTreeMap::from([(missing_component, &component)]), &mappings, coverage_limits), Err(ActionCoverageError::ComponentIdentityMismatch(id)) if id == missing_component)
        );
        let mut wrong_scope = sibling.identity().clone();
        wrong_scope.scope = SemanticScopeId::from_canonical_fingerprint(fp(46));
        let wrong_scope = ResidualObligation::new(
            wrong_scope,
            item.kind().clone(),
            item.bindings(),
            limits().identity,
        )
        .unwrap();
        let mut different = ResidualObligationSet::new(limits());
        different.insert(wrong_scope.clone()).unwrap();
        assert!(
            matches!(verify_action_component_coverage(&different, &components, &[ActionCoverageEntry { obligation: wrong_scope.id(), component: component.id() }], coverage_limits), Err(ActionCoverageError::IncompatibleComponent(id)) if id == wrong_scope.id())
        );
        let mut distinct_clause = ResidualObligationSet::new(limits());
        distinct_clause.insert(sibling.clone()).unwrap();
        assert!(matches!(
            verify_action_component_coverage(&distinct_clause, &components, &[ActionCoverageEntry { obligation: sibling.id(), component: component.id() }], coverage_limits),
            Err(ActionCoverageError::IncompatibleComponent(id)) if id == sibling.id()
        ));
        assert!(matches!(
            verify_action_component_coverage(&required, &components, &[ActionCoverageEntry { obligation: wrong_scope.id(), component: component.id() }], coverage_limits),
            Err(ActionCoverageError::UnexpectedObligation(id)) if id == wrong_scope.id()
        ));
        let extra = CompiledActionPredicate::compile(
            wrong_scope.clone(),
            pattern.clone(),
            predicate.clone(),
            layout.clone(),
            component_limits,
        )
        .unwrap();
        let with_unused = BTreeMap::from([(component.id(), &component), (extra.id(), &extra)]);
        assert!(matches!(
            verify_action_component_coverage(&required, &with_unused, &mappings, coverage_limits),
            Err(ActionCoverageError::UnusedComponent(id)) if id == extra.id()
        ));
        let mut tiny = coverage_limits;
        tiny.max_mappings = 0;
        assert!(matches!(
            verify_action_component_coverage(&required, &components, &mappings, tiny),
            Err(ActionCoverageError::MappingLimit)
        ));
        let unsupported = ResidualObligation::new(
            identity_for_rule(&item, ResidualRuleKind::TraceContract),
            ResidualKind::TraceContract {
                monitor: component.id(),
            },
            item.bindings(),
            limits().identity,
        )
        .unwrap();
        let mut unsupported_set = ResidualObligationSet::new(limits());
        unsupported_set.insert(unsupported.clone()).unwrap();
        assert!(
            matches!(verify_action_component_coverage(&unsupported_set, &components, &[], coverage_limits), Err(ActionCoverageError::UnsupportedKind(id)) if id == unsupported.id())
        );
        assert!(matches!(
            verify_action_component_coverage(
                &ResidualObligationSet::new(limits()),
                &components,
                &[],
                coverage_limits
            ),
            Err(ActionCoverageError::NoObligations)
        ));
        assert_eq!(
            component.evaluate(ActionComponentInput {
                scope: item.identity().scope,
                activation: item.identity().activation,
                source_revision: item.identity().origin.revision,
                action: pattern.effect(),
                bindings: &values,
            }),
            Ok(ActionPredicateDecision::Applicable(
                PredicateDecision::Satisfied
            ))
        );
        let unbound = BTreeMap::from([(other_value, BorrowedValue::Text("draft"))]);
        assert_eq!(
            component.evaluate(ActionComponentInput {
                scope: item.identity().scope,
                activation: item.identity().activation,
                source_revision: item.identity().origin.revision,
                action: pattern.effect(),
                bindings: &unbound,
            }),
            Ok(ActionPredicateDecision::Applicable(
                PredicateDecision::NeedsEvidence
            ))
        );
        let wrong_type = BTreeMap::from([(value, BorrowedValue::Bytes(b"draft"))]);
        assert_eq!(
            component.evaluate(ActionComponentInput {
                scope: item.identity().scope,
                activation: item.identity().activation,
                source_revision: item.identity().origin.revision,
                action: pattern.effect(),
                bindings: &wrong_type,
            }),
            Err(ActionPredicateError::Evaluation(
                PredicateEvaluationError::TypeMismatch(value)
            ))
        );
        assert!(matches!(
            CompiledActionPredicate::from_parts(
                MonitorTemplateId::from_canonical_fingerprint(fp(41)),
                item.clone(),
                pattern.clone(),
                predicate.clone(),
                layout.clone(),
                component_limits,
            ),
            Err(ActionComponentError::IdentityMismatch)
        ));
        let mut too_small_component = component_limits;
        too_small_component.evaluation.max_work = 0;
        assert!(matches!(
            CompiledActionPredicate::compile(
                item.clone(),
                pattern.clone(),
                predicate.clone(),
                layout.clone(),
                too_small_component
            ),
            Err(ActionComponentError::UnsupportedPredicate(
                crate::residual::predicate::BorrowedPredicateSupportError::WorkLimit
            ))
        ));
        let mut different_limits = component_limits;
        different_limits.evaluation.max_value_bytes -= 1;
        let changed = CompiledActionPredicate::compile(
            item.clone(),
            pattern.clone(),
            predicate.clone(),
            layout.clone(),
            different_limits,
        )
        .unwrap();
        assert_ne!(component.id(), changed.id());
        let value_pattern =
            crate::residual::identity::ValuePatternId::from_canonical_fingerprint(fp(42));
        let unsupported_node = PredicateNode::PatternMember {
            value,
            pattern: value_pattern,
        };
        let unsupported_id = PredicateId::from_node(&unsupported_node);
        let mut unsupported_schema = PredicateSchema::default();
        unsupported_schema.bindings.insert(value, ValueType::Text);
        unsupported_schema
            .patterns
            .insert(value_pattern, ValueType::Text);
        let unsupported = PredicateGraph {
            root: unsupported_id,
            nodes: BTreeMap::from([(unsupported_id, unsupported_node)]),
        }
        .bind(&unsupported_schema, component_limits.predicate)
        .unwrap();
        let unsupported_item = ResidualObligation::new(
            item.identity().clone(),
            ResidualKind::ActionPattern {
                pattern: pattern.id(),
                predicate: unsupported_id,
            },
            layout.id(),
            limits().identity,
        )
        .unwrap();
        assert!(
            matches!(CompiledActionPredicate::compile(unsupported_item, pattern.clone(), unsupported, layout.clone(), component_limits), Err(ActionComponentError::UnsupportedPredicate(crate::residual::predicate::BorrowedPredicateSupportError::UnsupportedPattern(id))) if id == unsupported_id)
        );
        let provider = GuardProviderId::from_canonical_fingerprint(fp(43));
        let evidence_schema = crate::residual::predicate::EvidenceTestSchema {
            provider,
            semantic_version: 1,
            arguments: vec![ValueType::Text],
        };
        let evidence_id = crate::residual::identity::EvidenceTestId::from_schema(&evidence_schema);
        let evidence_node = PredicateNode::Evidence {
            test: evidence_id,
            arguments: vec![value],
        };
        let evidence_root = PredicateId::from_node(&evidence_node);
        let mut evidence_schema_set = PredicateSchema::default();
        evidence_schema_set.bindings.insert(value, ValueType::Text);
        evidence_schema_set
            .evidence_tests
            .insert(evidence_id, evidence_schema);
        evidence_schema_set
            .registered_providers
            .insert((provider, 1));
        let evidence_predicate = PredicateGraph {
            root: evidence_root,
            nodes: BTreeMap::from([(evidence_root, evidence_node)]),
        }
        .bind(&evidence_schema_set, component_limits.predicate)
        .unwrap();
        let evidence_item = ResidualObligation::new(
            item.identity().clone(),
            ResidualKind::ActionPattern {
                pattern: pattern.id(),
                predicate: evidence_root,
            },
            layout.id(),
            limits().identity,
        )
        .unwrap();
        assert!(
            matches!(CompiledActionPredicate::compile(evidence_item, pattern.clone(), evidence_predicate, layout.clone(), component_limits), Err(ActionComponentError::UnsupportedPredicate(crate::residual::predicate::BorrowedPredicateSupportError::UnsupportedEvidence(id))) if id == evidence_root)
        );
        let result_binding = BindingDescriptor {
            root: BindingRoot::Result,
            fields: vec![],
            value_type: ValueType::Text,
        };
        let result_id = result_binding.id(2).unwrap();
        let result_layout =
            BindingLayout::new(BTreeMap::from([(result_id, result_binding)]), layout_limits)
                .unwrap();
        let result_node = PredicateNode::Equal {
            left: result_id,
            right: result_id,
        };
        let result_predicate_id = PredicateId::from_node(&result_node);
        let mut result_schema = PredicateSchema::default();
        result_schema.bindings.insert(result_id, ValueType::Text);
        let result_predicate = PredicateGraph {
            root: result_predicate_id,
            nodes: BTreeMap::from([(result_predicate_id, result_node)]),
        }
        .bind(&result_schema, component_limits.predicate)
        .unwrap();
        let result_item = ResidualObligation::new(
            item.identity().clone(),
            ResidualKind::ActionPattern {
                pattern: pattern.id(),
                predicate: result_predicate_id,
            },
            result_layout.id(),
            limits().identity,
        )
        .unwrap();
        assert!(
            matches!(CompiledActionPredicate::compile(result_item, pattern.clone(), result_predicate, result_layout, component_limits), Err(ActionComponentError::UnsupportedBinding(id)) if id == result_id)
        );
        assert_eq!(
            item.evaluate_action_pattern(input(pattern.effect(), &empty_values)),
            Ok(ActionPredicateDecision::Applicable(
                PredicateDecision::NeedsEvidence
            ))
        );
        assert_eq!(
            item.evaluate_action_pattern(input(other_pattern.effect(), &values)),
            Ok(ActionPredicateDecision::NotApplicable)
        );
        let mut wrong = input(pattern.effect(), &values);
        wrong.scope = SemanticScopeId::from_canonical_fingerprint(fp(30));
        assert_eq!(
            item.evaluate_action_pattern(wrong),
            Err(ActionPredicateError::ScopeMismatch)
        );
        let mut wrong = input(pattern.effect(), &values);
        wrong.activation = ActivationSiteId::from_canonical_fingerprint(fp(31));
        assert_eq!(
            item.evaluate_action_pattern(wrong),
            Err(ActionPredicateError::ActivationMismatch)
        );
        let mut wrong = input(pattern.effect(), &values);
        wrong.source_revision = SourceRevisionKey::new(item.identity().origin.source, b"different");
        assert_eq!(
            item.evaluate_action_pattern(wrong),
            Err(ActionPredicateError::RevisionMismatch)
        );
        let mut insufficient = input(pattern.effect(), &empty_values);
        insufficient.evaluation_limits.max_bindings = 0;
        assert_eq!(
            item.evaluate_action_pattern(insufficient),
            Err(ActionPredicateError::Evaluation(
                PredicateEvaluationError::BindingLimit
            ))
        );
        let mut uncertain_action = pattern.effect().clone();
        uncertain_action.arguments.push(CanonicalSelector::Wildcard);
        assert_eq!(
            item.evaluate_action_pattern(input(&uncertain_action, &values)),
            Err(ActionPredicateError::InvalidAction(
                ActionPatternError::SelectorArityMismatch
            ))
        );
        let mut specific_action = pattern.effect().clone();
        specific_action.arguments = vec![CanonicalSelector::String("example".into())];
        let specific_pattern = ActionPattern::new(specific_action.clone(), pattern_limits).unwrap();
        let specific_item = ResidualObligation::new(
            item.identity().clone(),
            ResidualKind::ActionPattern {
                pattern: specific_pattern.id(),
                predicate: predicate_id,
            },
            layout.id(),
            limits().identity,
        )
        .unwrap();
        specific_action.arguments = vec![CanonicalSelector::Wildcard];
        assert_eq!(
            specific_item.evaluate_action_pattern(ActionPredicateInput {
                scope: specific_item.identity().scope,
                activation: specific_item.identity().activation,
                source_revision: specific_item.identity().origin.revision,
                pattern: &specific_pattern,
                predicate: &predicate,
                layout: &layout,
                action: &specific_action,
                bindings: &values,
                pattern_limits,
                evaluation_limits: eval_limits,
            }),
            Ok(ActionPredicateDecision::NeedsConcreteSelector)
        );
        assert_eq!(
            item.validate_action_pattern_binding(&other_pattern, &predicate, &layout),
            Err(ResidualBindingError::ActionPatternMismatch)
        );
        let mut obligations = ResidualObligationSet::new(limits());
        obligations.insert(item.clone()).unwrap();
        let bounds = ResidualReferenceLimits {
            max_obligations: 1,
            max_binding_checks: 1,
        };
        let mut catalog = ResidualReferenceCatalog::default();
        assert_eq!(
            validate_references(&obligations, &catalog, bounds),
            Err(ResidualReferenceError::MissingPattern(pattern.id()))
        );
        catalog.patterns.insert(pattern.id(), &pattern);
        assert_eq!(
            validate_references(&obligations, &catalog, bounds),
            Err(ResidualReferenceError::MissingPredicate(predicate_id))
        );
        catalog.predicates.insert(predicate_id, &predicate);
        assert_eq!(
            validate_references(&obligations, &catalog, bounds),
            Err(ResidualReferenceError::MissingLayout(layout.id()))
        );
        catalog.layouts.insert(layout.id(), &layout);
        assert_eq!(validate_references(&obligations, &catalog, bounds), Ok(()));
        let mut too_small = bounds;
        too_small.max_binding_checks = 0;
        assert_eq!(
            validate_references(&obligations, &catalog, too_small),
            Err(ResidualReferenceError::BindingCheckLimit)
        );
        catalog.patterns.insert(pattern.id(), &other_pattern);
        assert_eq!(
            validate_references(&obligations, &catalog, bounds),
            Err(ResidualReferenceError::InvalidBinding {
                obligation: item.id(),
                reason: ResidualBindingError::ActionPatternMismatch
            })
        );
        assert_eq!(
            item.validate_predicate_binding(&predicate, &other_layout),
            Err(ResidualBindingError::LayoutMismatch)
        );
        let bad_node = PredicateNode::Equal {
            left: other_value,
            right: other_value,
        };
        let bad_id = PredicateId::from_node(&bad_node);
        let bad_graph = PredicateGraph {
            root: bad_id,
            nodes: BTreeMap::from([(bad_id, bad_node)]),
        };
        let mut bad_schema = PredicateSchema::default();
        bad_schema.bindings.insert(other_value, ValueType::Text);
        let bad_predicate = bad_graph
            .bind(
                &bad_schema,
                PredicateLimits {
                    max_nodes: 4,
                    max_edges: 4,
                    max_depth: 4,
                    max_evidence_arguments: 2,
                },
            )
            .unwrap();
        let wrong_type = ResidualObligation::new(
            base.identity().clone(),
            ResidualKind::ActionPattern {
                pattern: ActionPatternId::from_canonical_fingerprint(fp(9)),
                predicate: bad_id,
            },
            other_layout.id(),
            limits().identity,
        )
        .unwrap();
        assert_eq!(
            wrong_type.validate_predicate_binding(&bad_predicate, &other_layout),
            Err(ResidualBindingError::Predicate(
                PredicateError::TypeMismatch
            ))
        );
        assert_eq!(
            base.validate_predicate_binding(&predicate, &layout),
            Err(ResidualBindingError::PredicateMismatch)
        );
    }
}
