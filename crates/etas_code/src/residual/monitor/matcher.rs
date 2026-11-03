//! Concrete adapter for the same prior-evidence automaton used by analysis.
//! This is a pure component, not a scope installation or dispatch permit.

use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
};

use etas_utils::automaton::Matcher;

use super::{PriorEvidenceLabel, PriorEvidenceState, ScopeInvocationId, compile_prior_evidence};
use crate::{
    RevisionKey,
    contract::CanonicalEffect,
    residual::{
        binding::ValidatedBorrowedBindings,
        identity::{ActionPatternId, ActivationSiteId, SemanticScopeId},
        pattern::{
            ActionPattern, ActionPatternError, ActionPatternLimits, ActionPatternMatch,
            validate_action_instance,
        },
        predicate::BorrowedValue,
    },
};

/// Checked payload bindings remain borrowed; no secret or debug representation
/// is copied into the monitor state.
pub struct ConcreteActionAttempt<'a> {
    pub scope: SemanticScopeId,
    pub activation: ActivationSiteId,
    pub revision: RevisionKey,
    pub invocation: ScopeInvocationId,
    pub id: ActionAttemptId,
    pub cursor: SemanticEventCursor,
    pub action: &'a CanonicalEffect,
    pub bindings: ValidatedBorrowedBindings<'a, 'a>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionAttemptId([u8; 16]);

impl ActionAttemptId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemanticEventCursor(pub u64);

/// A provider-authenticated accepted outcome; payload correlation and provider
/// origin still require validation by the native evidence adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcceptedPriorEvidence {
    pub scope: SemanticScopeId,
    pub activation: ActivationSiteId,
    pub revision: RevisionKey,
    pub invocation: ScopeInvocationId,
    pub guard_pattern: ActionPatternId,
    pub target_pattern: ActionPatternId,
    pub guard_attempt: ActionAttemptId,
    pub completed_at: SemanticEventCursor,
}

pub struct EvidenceQuery<'a, 'value> {
    pub guard: &'a ActionPattern,
    pub target: &'a ActionPattern,
    pub attempt: &'a ConcreteActionAttempt<'value>,
}

/// Identifies a pending checked prior-evidence query without retaining payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PriorEvidenceRequirement {
    pub guard: ActionPatternId,
    pub target: ActionPatternId,
}

/// `Accepted` must be returned only after the provider verifies origin,
/// correlation, scope, validity and the completed accepted outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceStatus {
    Accepted(AcceptedPriorEvidence),
    Absent,
    Denied,
    Failed,
    Pending,
    Unavailable,
}

/// A pure, bounded view over evidence already supplied by the native adapter.
/// Implementing this trait does not itself confer trust or installation proof.
pub trait ConcretePriorEvidence {
    fn query(&self, query: EvidenceQuery<'_, '_>) -> EvidenceStatus;
}

#[derive(Clone, Copy, Debug)]
pub struct ConcreteEvidenceLimits {
    pub pattern: ActionPatternLimits,
    pub max_bindings: usize,
    pub max_value_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorEvidenceStep {
    Active,
    Rejected,
    NeedsEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorEvidenceStepError {
    InvalidPattern(ActionPatternError),
    UnresolvedSelector,
    EvidenceUnavailable,
    InvalidEvidence,
    InvalidAutomaton,
    MissingPattern,
    BindingLimit,
    ValueLimit,
}

pub(super) struct ConcreteMatcher<'a, E> {
    evidence: &'a E,
    limits: ActionPatternLimits,
    issue: Cell<Option<PriorEvidenceStepError>>,
    pending: RefCell<BTreeSet<PriorEvidenceRequirement>>,
    patterns: Option<&'a BTreeMap<ActionPatternId, ActionPattern>>,
}

impl<'a, E> ConcreteMatcher<'a, E> {
    pub(super) fn new(evidence: &'a E, limits: ActionPatternLimits) -> Self {
        Self {
            evidence,
            limits,
            issue: Cell::new(None),
            pending: RefCell::new(BTreeSet::new()),
            patterns: None,
        }
    }

    pub(super) fn with_patterns(
        evidence: &'a E,
        limits: ActionPatternLimits,
        patterns: &'a BTreeMap<ActionPatternId, ActionPattern>,
    ) -> Self {
        let mut matcher = Self::new(evidence, limits);
        matcher.patterns = Some(patterns);
        matcher
    }

    pub(super) fn issue(&self) -> Option<PriorEvidenceStepError> {
        self.issue.get()
    }
    pub(super) fn pending(&self) -> bool {
        !self.pending.borrow().is_empty()
    }
    pub(super) fn pending_requirements(&self) -> Vec<PriorEvidenceRequirement> {
        self.pending.borrow().iter().copied().collect()
    }
}

impl<E: ConcretePriorEvidence> ConcreteMatcher<'_, E> {
    fn matches_patterns(
        &self,
        guard: &ActionPattern,
        target: &ActionPattern,
        attempt: &ConcreteActionAttempt<'_>,
    ) -> bool {
        match target.match_action(attempt.action, self.limits) {
            Ok(ActionPatternMatch::Disjoint) => return false,
            Ok(ActionPatternMatch::NeedsConcreteSelector) => {
                self.issue
                    .set(Some(PriorEvidenceStepError::UnresolvedSelector));
                return false;
            }
            Err(error) => {
                self.issue
                    .set(Some(PriorEvidenceStepError::InvalidPattern(error)));
                return false;
            }
            Ok(ActionPatternMatch::Matches) => {}
        }
        match self.evidence.query(EvidenceQuery {
            guard,
            target,
            attempt,
        }) {
            EvidenceStatus::Accepted(evidence) => {
                if evidence.scope != attempt.scope
                    || evidence.activation != attempt.activation
                    || evidence.revision != attempt.revision
                    || evidence.invocation != attempt.invocation
                    || evidence.guard_pattern != guard.id()
                    || evidence.target_pattern != target.id()
                    || evidence.guard_attempt == attempt.id
                    || evidence.completed_at >= attempt.cursor
                {
                    self.issue
                        .set(Some(PriorEvidenceStepError::InvalidEvidence));
                }
                false
            }
            EvidenceStatus::Absent | EvidenceStatus::Denied | EvidenceStatus::Failed => true,
            EvidenceStatus::Pending => {
                self.pending.borrow_mut().insert(PriorEvidenceRequirement {
                    guard: guard.id(),
                    target: target.id(),
                });
                false
            }
            EvidenceStatus::Unavailable => {
                self.issue
                    .set(Some(PriorEvidenceStepError::EvidenceUnavailable));
                false
            }
        }
    }
}

impl<E: ConcretePriorEvidence> Matcher<PriorEvidenceLabel<ActionPattern>, ConcreteActionAttempt<'_>>
    for ConcreteMatcher<'_, E>
{
    fn matches(
        &self,
        label: &PriorEvidenceLabel<ActionPattern>,
        attempt: &ConcreteActionAttempt<'_>,
    ) -> bool {
        let PriorEvidenceLabel::Missing { guard, target } = label;
        self.matches_patterns(guard, target, attempt)
    }
}

impl<E: ConcretePriorEvidence>
    Matcher<PriorEvidenceLabel<ActionPatternId>, ConcreteActionAttempt<'_>>
    for ConcreteMatcher<'_, E>
{
    fn matches(
        &self,
        label: &PriorEvidenceLabel<ActionPatternId>,
        attempt: &ConcreteActionAttempt<'_>,
    ) -> bool {
        let PriorEvidenceLabel::Missing { guard, target } = label;
        let Some((guard, target)) = self
            .patterns
            .and_then(|patterns| Some((patterns.get(guard)?, patterns.get(target)?)))
        else {
            self.issue.set(Some(PriorEvidenceStepError::MissingPattern));
            return false;
        };
        self.matches_patterns(guard, target, attempt)
    }
}

/// Evaluates one checked pre-dispatch attempt. A pending result is not a permit;
/// the caller must obtain evidence and rerun this operation before dispatch.
pub fn step_concrete_prior_evidence<E: ConcretePriorEvidence>(
    guard: &ActionPattern,
    target: &ActionPattern,
    state: PriorEvidenceState,
    attempt: &ConcreteActionAttempt<'_>,
    evidence: &E,
    limits: ConcreteEvidenceLimits,
) -> Result<PriorEvidenceStep, PriorEvidenceStepError> {
    validate_attempt(attempt, limits)?;
    guard
        .validate_with_limits(limits.pattern)
        .map_err(PriorEvidenceStepError::InvalidPattern)?;
    target
        .validate_with_limits(limits.pattern)
        .map_err(PriorEvidenceStepError::InvalidPattern)?;
    let automaton = compile_prior_evidence(guard, target);
    let matcher = ConcreteMatcher::new(evidence, limits.pattern);
    let step = automaton.step(&state, attempt, &matcher);
    if let Some(error) = matcher.issue() {
        return Err(error);
    }
    if matcher.pending() {
        return Ok(PriorEvidenceStep::NeedsEvidence);
    }
    if !step.rejecting.is_empty() {
        return Ok(PriorEvidenceStep::Rejected);
    }
    if step.states.len() == 1 && step.accepting.contains(&PriorEvidenceState::Active) {
        return Ok(PriorEvidenceStep::Active);
    }
    Err(PriorEvidenceStepError::InvalidAutomaton)
}

pub(super) fn validate_attempt(
    attempt: &ConcreteActionAttempt<'_>,
    limits: ConcreteEvidenceLimits,
) -> Result<(), PriorEvidenceStepError> {
    if attempt.bindings.supplied_count() > limits.max_bindings {
        return Err(PriorEvidenceStepError::BindingLimit);
    }
    let mut bytes = 0usize;
    for value in attempt.bindings.checked_values() {
        let length = match value {
            BorrowedValue::Text(value) => value.len(),
            BorrowedValue::Bytes(value) | BorrowedValue::Identity { value, .. } => value.len(),
            BorrowedValue::Boolean(_) => 1,
            BorrowedValue::Signed { .. } | BorrowedValue::Unsigned { .. } => 16,
        };
        bytes = bytes
            .checked_add(length)
            .ok_or(PriorEvidenceStepError::ValueLimit)?;
        if bytes > limits.max_value_bytes {
            return Err(PriorEvidenceStepError::ValueLimit);
        }
    }
    validate_action_instance(attempt.action, limits.pattern)
        .map_err(PriorEvidenceStepError::InvalidPattern)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CodeFingerprint, DefinitionKey, DefinitionKind, PackageKey,
        contract::CanonicalSelector,
        descriptor::EffectTargetInfo,
        residual::{
            binding::{
                BindingDescriptor, BindingLayout, BindingLayoutLimits, BindingRoot,
                BorrowedBindingLimits, ValidatedBorrowedBindings,
            },
            identity::ValueBindingId,
            predicate::ValueType,
        },
    };

    fn binding_view<'a>(
        layout: &'a BindingLayout,
        values: &'a BTreeMap<ValueBindingId, BorrowedValue<'a>>,
    ) -> ValidatedBorrowedBindings<'a, 'a> {
        layout
            .validate_borrowed(
                values,
                BorrowedBindingLimits {
                    max_bindings: 8,
                    max_value_bytes: 1024,
                    max_total_value_bytes: 8192,
                },
            )
            .unwrap()
    }

    fn layout(bindings: BTreeMap<ValueBindingId, BindingDescriptor>) -> BindingLayout {
        BindingLayout::new(
            bindings,
            BindingLayoutLimits {
                max_bindings: 8,
                max_projections_per_binding: 2,
                max_total_projections: 8,
            },
        )
        .unwrap()
    }

    fn fingerprint(byte: u8) -> CodeFingerprint {
        CodeFingerprint::from_bytes([byte; 32])
    }

    fn limits() -> ActionPatternLimits {
        ActionPatternLimits {
            max_selectors: 4,
            max_identity_segments: 8,
            max_identity_bytes: 80,
            max_literal_bytes: 80,
        }
    }

    fn evidence_limits() -> ConcreteEvidenceLimits {
        ConcreteEvidenceLimits {
            pattern: limits(),
            max_bindings: 8,
            max_value_bytes: 64,
        }
    }

    fn action(name: &str) -> CanonicalEffect {
        let effect = DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(fingerprint(1)),
            vec!["app".into()],
            vec!["Control".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        CanonicalEffect {
            target: EffectTargetInfo::Action {
                effect,
                name: name.into(),
            },
            arguments: vec![CanonicalSelector::String("tenant-a".into())],
        }
    }

    struct Evidence {
        status: EvidenceStatus,
        expected_scope: SemanticScopeId,
        expected_activation: ActivationSiteId,
        expected_revision: RevisionKey,
        expected_invocation: ScopeInvocationId,
        expected_binding: ValueBindingId,
        calls: Cell<usize>,
    }

    impl ConcretePriorEvidence for Evidence {
        fn query(&self, query: EvidenceQuery<'_, '_>) -> EvidenceStatus {
            self.calls.set(self.calls.get() + 1);
            if query.attempt.scope != self.expected_scope
                || query.attempt.activation != self.expected_activation
                || query.attempt.revision != self.expected_revision
                || query.attempt.invocation != self.expected_invocation
                || !matches!(
                    query.attempt.bindings.get(self.expected_binding),
                    Some(BorrowedValue::Text("draft-a"))
                )
                || query.guard.effect() != &action("approve")
                || query.target.effect() != &action("send")
            {
                return EvidenceStatus::Absent;
            }
            self.status
        }
    }

    #[test]
    fn only_correlated_accepted_evidence_satisfies_prior_requirement() {
        let guard = ActionPattern::new(action("approve"), limits()).unwrap();
        let target = ActionPattern::new(action("send"), limits()).unwrap();
        let scope = SemanticScopeId::from_canonical_fingerprint(fingerprint(2));
        let activation = ActivationSiteId::from_canonical_fingerprint(fingerprint(3));
        let revision = RevisionKey::from_canonical_fingerprint(fingerprint(4));
        let invocation = ScopeInvocationId::from_bytes([9; 16]);
        let descriptor = BindingDescriptor {
            root: BindingRoot::Parameter { position: 0 },
            fields: vec![],
            value_type: ValueType::Text,
        };
        let binding = descriptor.id(2).unwrap();
        let layout = layout(BTreeMap::from([(binding, descriptor)]));
        let bindings = BTreeMap::from([(binding, BorrowedValue::Text("draft-a"))]);
        let send = action("send");
        let attempt = ConcreteActionAttempt {
            scope,
            activation,
            revision,
            invocation,
            id: ActionAttemptId::from_bytes([7; 16]),
            cursor: SemanticEventCursor(2),
            action: &send,
            bindings: binding_view(&layout, &bindings),
        };
        let accepted = AcceptedPriorEvidence {
            scope,
            activation,
            revision,
            invocation,
            guard_pattern: guard.id(),
            target_pattern: target.id(),
            guard_attempt: ActionAttemptId::from_bytes([6; 16]),
            completed_at: SemanticEventCursor(1),
        };
        for (status, expected) in [
            (
                EvidenceStatus::Accepted(accepted),
                Ok(PriorEvidenceStep::Active),
            ),
            (EvidenceStatus::Absent, Ok(PriorEvidenceStep::Rejected)),
            (EvidenceStatus::Denied, Ok(PriorEvidenceStep::Rejected)),
            (EvidenceStatus::Failed, Ok(PriorEvidenceStep::Rejected)),
            (
                EvidenceStatus::Pending,
                Ok(PriorEvidenceStep::NeedsEvidence),
            ),
            (
                EvidenceStatus::Unavailable,
                Err(PriorEvidenceStepError::EvidenceUnavailable),
            ),
        ] {
            let evidence = Evidence {
                status,
                expected_scope: scope,
                expected_activation: activation,
                expected_revision: revision,
                expected_invocation: invocation,
                expected_binding: binding,
                calls: Cell::new(0),
            };
            assert_eq!(
                step_concrete_prior_evidence(
                    &guard,
                    &target,
                    PriorEvidenceState::Active,
                    &attempt,
                    &evidence,
                    evidence_limits()
                ),
                expected
            );
            assert_eq!(evidence.calls.get(), 1);
        }
        for invalid in [
            AcceptedPriorEvidence {
                completed_at: SemanticEventCursor(2),
                ..accepted
            },
            AcceptedPriorEvidence {
                completed_at: SemanticEventCursor(3),
                ..accepted
            },
            AcceptedPriorEvidence {
                guard_attempt: attempt.id,
                ..accepted
            },
            AcceptedPriorEvidence {
                invocation: ScopeInvocationId::from_bytes([8; 16]),
                ..accepted
            },
            AcceptedPriorEvidence {
                scope: SemanticScopeId::from_canonical_fingerprint(fingerprint(8)),
                ..accepted
            },
            AcceptedPriorEvidence {
                activation: ActivationSiteId::from_canonical_fingerprint(fingerprint(8)),
                ..accepted
            },
            AcceptedPriorEvidence {
                revision: RevisionKey::from_canonical_fingerprint(fingerprint(8)),
                ..accepted
            },
            AcceptedPriorEvidence {
                guard_pattern: target.id(),
                ..accepted
            },
            AcceptedPriorEvidence {
                target_pattern: guard.id(),
                ..accepted
            },
        ] {
            let evidence = Evidence {
                status: EvidenceStatus::Accepted(invalid),
                expected_scope: scope,
                expected_activation: activation,
                expected_revision: revision,
                expected_invocation: invocation,
                expected_binding: binding,
                calls: Cell::new(0),
            };
            assert_eq!(
                step_concrete_prior_evidence(
                    &guard,
                    &target,
                    PriorEvidenceState::Active,
                    &attempt,
                    &evidence,
                    evidence_limits()
                ),
                Err(PriorEvidenceStepError::InvalidEvidence)
            );
        }
        let bounded_evidence = Evidence {
            status: EvidenceStatus::Accepted(accepted),
            expected_scope: scope,
            expected_activation: activation,
            expected_revision: revision,
            expected_invocation: invocation,
            expected_binding: binding,
            calls: Cell::new(0),
        };
        let mut bound = evidence_limits();
        bound.max_bindings = 0;
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Active,
                &attempt,
                &bounded_evidence,
                bound
            ),
            Err(PriorEvidenceStepError::BindingLimit)
        );
        bound.max_bindings = 1;
        bound.max_value_bytes = 6;
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Active,
                &attempt,
                &bounded_evidence,
                bound
            ),
            Err(PriorEvidenceStepError::ValueLimit)
        );
        assert_eq!(bounded_evidence.calls.get(), 0);
        let evidence = Evidence {
            status: EvidenceStatus::Accepted(accepted),
            expected_scope: SemanticScopeId::from_canonical_fingerprint(fingerprint(6)),
            expected_activation: activation,
            expected_revision: revision,
            expected_invocation: invocation,
            expected_binding: binding,
            calls: Cell::new(0),
        };
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Active,
                &attempt,
                &evidence,
                evidence_limits()
            ),
            Ok(PriorEvidenceStep::Rejected)
        );
        let wrong_bindings = BTreeMap::from([(binding, BorrowedValue::Text("draft-b"))]);
        let different = ConcreteActionAttempt {
            bindings: binding_view(&layout, &wrong_bindings),
            ..attempt
        };
        let evidence = Evidence {
            expected_scope: scope,
            ..evidence
        };
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Active,
                &different,
                &evidence,
                evidence_limits()
            ),
            Ok(PriorEvidenceStep::Rejected)
        );
    }

    #[test]
    fn evidence_provider_cannot_read_values_outside_checked_layout() {
        let guard = ActionPattern::new(action("approve"), limits()).unwrap();
        let target = ActionPattern::new(action("send"), limits()).unwrap();
        let scope = SemanticScopeId::from_canonical_fingerprint(fingerprint(2));
        let activation = ActivationSiteId::from_canonical_fingerprint(fingerprint(3));
        let revision = RevisionKey::from_canonical_fingerprint(fingerprint(4));
        let invocation = ScopeInvocationId::from_bytes([9; 16]);
        let checked = BindingDescriptor {
            root: BindingRoot::Parameter { position: 0 },
            fields: vec![],
            value_type: ValueType::Text,
        };
        let other = BindingDescriptor {
            root: BindingRoot::Parameter { position: 1 },
            fields: vec![],
            value_type: ValueType::Text,
        };
        let checked_id = checked.id(2).unwrap();
        let other_id = other.id(2).unwrap();
        let layout = layout(BTreeMap::from([(checked_id, checked)]));
        let values = BTreeMap::from([
            (checked_id, BorrowedValue::Text("visible")),
            (other_id, BorrowedValue::Text("draft-a")),
        ]);
        let send = action("send");
        let attempt = ConcreteActionAttempt {
            scope,
            activation,
            revision,
            invocation,
            id: ActionAttemptId::from_bytes([7; 16]),
            cursor: SemanticEventCursor(2),
            action: &send,
            bindings: binding_view(&layout, &values),
        };
        let evidence = Evidence {
            status: EvidenceStatus::Accepted(AcceptedPriorEvidence {
                scope,
                activation,
                revision,
                invocation,
                guard_pattern: guard.id(),
                target_pattern: target.id(),
                guard_attempt: ActionAttemptId::from_bytes([6; 16]),
                completed_at: SemanticEventCursor(1),
            }),
            expected_scope: scope,
            expected_activation: activation,
            expected_revision: revision,
            expected_invocation: invocation,
            expected_binding: other_id,
            calls: Cell::new(0),
        };
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Active,
                &attempt,
                &evidence,
                evidence_limits()
            ),
            Ok(PriorEvidenceStep::Rejected)
        );
        assert_eq!(evidence.calls.get(), 1);
    }

    #[test]
    fn unrelated_events_do_not_require_evidence_and_unknown_selectors_fail_closed() {
        let guard = ActionPattern::new(action("approve"), limits()).unwrap();
        let target = ActionPattern::new(action("send"), limits()).unwrap();
        let scope = SemanticScopeId::from_canonical_fingerprint(fingerprint(2));
        let activation = ActivationSiteId::from_canonical_fingerprint(fingerprint(3));
        let revision = RevisionKey::from_canonical_fingerprint(fingerprint(4));
        let invocation = ScopeInvocationId::from_bytes([9; 16]);
        let binding = ValueBindingId::from_canonical_fingerprint(fingerprint(5));
        let bindings = BTreeMap::new();
        let layout = layout(BTreeMap::new());
        let unrelated = action("other");
        let attempt = ConcreteActionAttempt {
            scope,
            activation,
            revision,
            invocation,
            id: ActionAttemptId::from_bytes([7; 16]),
            cursor: SemanticEventCursor(2),
            action: &unrelated,
            bindings: binding_view(&layout, &bindings),
        };
        let evidence = Evidence {
            status: EvidenceStatus::Absent,
            expected_scope: scope,
            expected_activation: activation,
            expected_revision: revision,
            expected_invocation: invocation,
            expected_binding: binding,
            calls: Cell::new(0),
        };
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Active,
                &attempt,
                &evidence,
                evidence_limits()
            ),
            Ok(PriorEvidenceStep::Active)
        );
        assert_eq!(evidence.calls.get(), 0);
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Rejected,
                &attempt,
                &evidence,
                evidence_limits()
            ),
            Ok(PriorEvidenceStep::Rejected)
        );
        let mut unresolved = action("send");
        unresolved.arguments = vec![CanonicalSelector::Wildcard];
        let attempt = ConcreteActionAttempt {
            action: &unresolved,
            ..attempt
        };
        assert_eq!(
            step_concrete_prior_evidence(
                &guard,
                &target,
                PriorEvidenceState::Active,
                &attempt,
                &evidence,
                evidence_limits()
            ),
            Err(PriorEvidenceStepError::UnresolvedSelector)
        );
    }
}
