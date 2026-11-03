//! Pure per-scope OR state for a restricted action/previous-evidence component.
//! Native plan validation, site binding and dispatch installation remain separate.

use std::collections::{BTreeMap, BTreeSet};

use etas_utils::automaton::{ProductAutomaton, ProductState, ProductStepLimits};

use crate::{
    CodeFingerprint, RevisionKey,
    identity::canonical::CanonicalEncoder,
    residual::{
        identity::{ActionPatternId, ActivationSiteId, MonitorTemplateId, SemanticScopeId},
        pattern::{ActionPattern, ActionPatternError, ActionPatternMatch},
    },
};

use super::{
    ConcreteActionAttempt, ConcreteEvidenceLimits, ConcretePriorEvidence, MAX_TRACE_TEMPLATE_NODES,
    PriorEvidenceLabel, PriorEvidenceRequirement, PriorEvidenceState, PriorEvidenceStepError,
    PriorEvidenceTemplate, PriorEvidenceTemplateError, SemanticEventCursor, TraceAlternativeClause,
    TraceAlternativesTemplate, TraceClause,
    matcher::{ConcreteMatcher, validate_attempt},
    normalize_trace_alternative,
};

#[derive(Clone, Debug)]
pub enum AlternativeRule {
    Allow(ActionPattern),
    Deny(ActionPattern),
    Before {
        guard: ActionPattern,
        target: ActionPattern,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct AlternativeLimits {
    pub evidence: ConcreteEvidenceLimits,
    pub max_alternatives: usize,
    pub max_clauses: usize,
    pub max_patterns: usize,
    pub max_checks_per_event: usize,
    pub max_product_work_per_event: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopeInvocationId([u8; 16]);

impl ScopeInvocationId {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlternativeScope {
    pub scope: SemanticScopeId,
    pub activation: ActivationSiteId,
    pub revision: RevisionKey,
    pub invocation: ScopeInvocationId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlternativeError {
    EmptyAlternatives,
    EmptyAlternative,
    AlternativeLimit,
    ClauseLimit,
    PatternLimit,
    CheckLimit,
    ProductWorkLimit,
    InvalidProduct,
    InvalidPendingEvidence,
    Pattern(ActionPatternError),
    Prior(PriorEvidenceStepError),
    Template(PriorEvidenceTemplateError),
    PatternIdentityConflict,
    ScopeMismatch,
    PlanMismatch,
    StaleTransition,
    StaleEvent,
    VersionExhausted,
    LimitEncoding,
}

#[derive(Clone)]
struct Alternative {
    permissions: Vec<ActionPatternId>,
    denials: Vec<ActionPatternId>,
    prior: Vec<PriorEvidenceTemplate>,
    product: ProductAutomaton<usize, PriorEvidenceState, PriorEvidenceLabel<ActionPatternId>>,
}

enum RuleIds {
    Allow(ActionPatternId),
    Deny(ActionPatternId),
    Before(ActionPatternId, ActionPatternId),
}

/// This component handles only normalized static action patterns and prior
/// evidence. It does not assert full plan coverage or backend support.
pub struct CompiledTraceAlternatives {
    id: CodeFingerprint,
    template_id: MonitorTemplateId,
    template: TraceAlternativesTemplate,
    alternatives: Vec<Alternative>,
    patterns: BTreeMap<ActionPatternId, ActionPattern>,
    limits: AlternativeLimits,
}

pub struct AlternativeState {
    id: CodeFingerprint,
    scope: AlternativeScope,
    version: u64,
    last_cursor: Option<SemanticEventCursor>,
    active: BTreeMap<usize, ProductState<usize, PriorEvidenceState>>,
}

pub struct AlternativeDelta {
    id: CodeFingerprint,
    scope: AlternativeScope,
    version: u64,
    cursor: SemanticEventCursor,
    active: BTreeMap<usize, ProductState<usize, PriorEvidenceState>>,
}

/// A pending branch is reported independently; requirements from different
/// alternatives must never be combined into one authorization condition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingAlternativeEvidence {
    alternative: usize,
    requirements: Vec<PriorEvidenceRequirement>,
}

impl PendingAlternativeEvidence {
    pub fn alternative(&self) -> usize {
        self.alternative
    }
    pub fn requirements(&self) -> &[PriorEvidenceRequirement] {
        &self.requirements
    }
}

/// A request to re-evaluate one event after evidence arrives, not a permit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRequirements {
    plan: CodeFingerprint,
    scope: AlternativeScope,
    state_version: u64,
    attempt: super::ActionAttemptId,
    cursor: SemanticEventCursor,
    alternatives: Vec<PendingAlternativeEvidence>,
}

impl EvidenceRequirements {
    pub fn plan(&self) -> CodeFingerprint {
        self.plan
    }
    pub fn scope(&self) -> AlternativeScope {
        self.scope
    }
    pub fn state_version(&self) -> u64 {
        self.state_version
    }
    pub fn attempt(&self) -> super::ActionAttemptId {
        self.attempt
    }
    pub fn cursor(&self) -> SemanticEventCursor {
        self.cursor
    }
    pub fn alternatives(&self) -> &[PendingAlternativeEvidence] {
        &self.alternatives
    }
}

pub enum AlternativeDecision {
    Permit(AlternativeDelta),
    NeedsEvidence(EvidenceRequirements),
    Reject,
}

/// This classifies the monitor constraint only, not the execution outcome or
/// cleanup status of the enclosing scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeExit {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeCheckResult {
    Accepting,
    Rejected,
    PendingEvidence,
    Incomplete,
}

impl CompiledTraceAlternatives {
    pub fn compile(
        rules: &[Vec<AlternativeRule>],
        limits: AlternativeLimits,
    ) -> Result<Self, AlternativeError> {
        if rules.is_empty() {
            return Err(AlternativeError::EmptyAlternatives);
        }
        if rules.len() > limits.max_alternatives {
            return Err(AlternativeError::AlternativeLimit);
        }
        if rules.len() > MAX_TRACE_TEMPLATE_NODES {
            return Err(AlternativeError::AlternativeLimit);
        }
        let mut count = 0usize;
        let mut patterns = BTreeMap::new();
        let mut template_groups = Vec::with_capacity(rules.len());
        for group in rules {
            if group.is_empty() {
                return Err(AlternativeError::EmptyAlternative);
            }
            count = count
                .checked_add(group.len())
                .ok_or(AlternativeError::ClauseLimit)?;
            if count > limits.max_clauses
                || count
                    .checked_add(rules.len())
                    .is_none_or(|n| n > MAX_TRACE_TEMPLATE_NODES)
            {
                return Err(AlternativeError::ClauseLimit);
            }
            let mut ids = Vec::with_capacity(group.len());
            for rule in group {
                let mut add =
                    |pattern: &ActionPattern| -> Result<ActionPatternId, AlternativeError> {
                        pattern
                            .validate_with_limits(limits.evidence.pattern)
                            .map_err(AlternativeError::Pattern)?;
                        let id = pattern.id();
                        if let Some(existing) = patterns.get(&id) {
                            if existing != pattern {
                                return Err(AlternativeError::PatternIdentityConflict);
                            }
                        } else {
                            if patterns.len() >= limits.max_patterns {
                                return Err(AlternativeError::PatternLimit);
                            }
                            patterns.insert(id, pattern.clone());
                        }
                        Ok(id)
                    };
                ids.push(match rule {
                    AlternativeRule::Allow(pattern) => RuleIds::Allow(add(pattern)?),
                    AlternativeRule::Deny(pattern) => RuleIds::Deny(add(pattern)?),
                    AlternativeRule::Before { guard, target } => {
                        RuleIds::Before(add(guard)?, add(target)?)
                    }
                });
            }
            template_groups.push(
                ids.iter()
                    .map(|rule| match rule {
                        RuleIds::Allow(pattern) => TraceAlternativeClause::Allow(*pattern),
                        RuleIds::Deny(pattern) => TraceAlternativeClause::Deny(*pattern),
                        RuleIds::Before(guard, target) => TraceAlternativeClause::Before(
                            PriorEvidenceTemplate::new(*guard, *target),
                        ),
                    })
                    .collect(),
            );
        }
        let template = TraceAlternativesTemplate::new(template_groups);
        Self::from_template(&template, &patterns, limits)
    }

    /// Rebuilds from portable data and checked patterns. This does not verify a
    /// complete residual plan or grant permission to dispatch an action.
    pub fn from_template(
        template: &TraceAlternativesTemplate,
        patterns: &BTreeMap<ActionPatternId, ActionPattern>,
        limits: AlternativeLimits,
    ) -> Result<Self, AlternativeError> {
        template
            .validate_version()
            .map_err(AlternativeError::Template)?;
        let groups = template.alternatives();
        if groups.len() > limits.max_alternatives || groups.len() > MAX_TRACE_TEMPLATE_NODES {
            return Err(AlternativeError::AlternativeLimit);
        }
        let mut count = groups.len();
        if groups.is_empty() {
            return Err(AlternativeError::EmptyAlternatives);
        }
        let mut alternatives = Vec::with_capacity(groups.len());
        let mut checked_patterns = BTreeMap::new();
        for group in groups {
            if group.is_empty() {
                return Err(AlternativeError::EmptyAlternative);
            }
            count = count
                .checked_add(group.len())
                .ok_or(AlternativeError::ClauseLimit)?;
            if count > MAX_TRACE_TEMPLATE_NODES || count - groups.len() > limits.max_clauses {
                return Err(AlternativeError::ClauseLimit);
            }
            let mut resolve = |id| -> Result<ActionPatternId, AlternativeError> {
                let pattern = patterns.get(&id).ok_or(AlternativeError::Template(
                    PriorEvidenceTemplateError::MissingPattern(id),
                ))?;
                if pattern.id() != id {
                    return Err(AlternativeError::Template(
                        PriorEvidenceTemplateError::PatternIdentityMismatch(id),
                    ));
                }
                if !checked_patterns.contains_key(&id) {
                    if checked_patterns.len() >= limits.max_patterns {
                        return Err(AlternativeError::PatternLimit);
                    }
                    pattern
                        .validate_with_limits(limits.evidence.pattern)
                        .map_err(AlternativeError::Pattern)?;
                    checked_patterns.insert(id, pattern.clone());
                }
                Ok(id)
            };
            let mut ids = Vec::with_capacity(group.len());
            for clause in group {
                ids.push(match clause {
                    TraceAlternativeClause::Allow(id) => RuleIds::Allow(resolve(*id)?),
                    TraceAlternativeClause::Deny(id) => RuleIds::Deny(resolve(*id)?),
                    TraceAlternativeClause::Before(prior) => {
                        prior
                            .validate_version()
                            .map_err(AlternativeError::Template)?;
                        RuleIds::Before(resolve(prior.guard())?, resolve(prior.target())?)
                    }
                });
            }
            let clauses = ids.iter().map(|rule| match rule {
                RuleIds::Allow(pattern) => TraceClause::Allow {
                    pattern,
                    origin: (),
                },
                RuleIds::Deny(pattern) => TraceClause::Deny {
                    pattern,
                    origin: (),
                },
                RuleIds::Before(guard, target) => TraceClause::Before {
                    guard,
                    target,
                    origin: (),
                },
            });
            let normalized = normalize_trace_alternative(clauses, group.len())
                .map_err(|_| AlternativeError::ClauseLimit)?;
            let prior: Vec<_> = normalized
                .prerequisites
                .into_iter()
                .map(|rule| PriorEvidenceTemplate::new(*rule.guard, *rule.target))
                .collect();
            let mut product = ProductAutomaton::new();
            for (index, prior_template) in prior.iter().enumerate() {
                let automaton = prior_template
                    .rebuild(&checked_patterns, limits.evidence.pattern)
                    .map_err(AlternativeError::Template)?;
                product.insert(index, automaton);
            }
            alternatives.push(Alternative {
                permissions: normalized
                    .permissions
                    .into_iter()
                    .map(|(id, ())| *id)
                    .collect(),
                denials: normalized.denials.into_iter().map(|(id, ())| *id).collect(),
                prior,
                product,
            });
        }
        let id = fingerprint(template, &alternatives, limits)?;
        let template_id = semantic_template_id(template, &alternatives)?;
        Ok(Self {
            id,
            template_id,
            template: template.clone(),
            alternatives,
            patterns: checked_patterns,
            limits,
        })
    }

    pub fn template(&self) -> &TraceAlternativesTemplate {
        &self.template
    }

    /// The validated canonical patterns referenced by `template`. They are
    /// inputs for binding this monitor to an obligation, not coverage proof.
    pub fn patterns(&self) -> &BTreeMap<ActionPatternId, ActionPattern> {
        &self.patterns
    }

    /// Semantic identity of the validated normalized alternatives. It is not
    /// a plan-coverage proof or an engine installation token.
    pub fn template_id(&self) -> MonitorTemplateId {
        self.template_id
    }

    pub(super) fn runtime_id(&self) -> CodeFingerprint {
        self.id
    }

    pub fn initial_state(&self, scope: AlternativeScope) -> AlternativeState {
        AlternativeState {
            id: self.id,
            scope,
            version: 0,
            last_cursor: None,
            active: self
                .alternatives
                .iter()
                .enumerate()
                .map(|(index, alternative)| (index, alternative.product.initial_state()))
                .collect(),
        }
    }

    /// `pending` must be the complete engine-owned set for this scope. This
    /// pure check does not establish installation or execution success.
    pub fn finish_scope(
        &self,
        state: &AlternativeState,
        pending: &[EvidenceRequirements],
        exit: ScopeExit,
    ) -> Result<ScopeCheckResult, AlternativeError> {
        self.finish_scope_refs(state, pending.iter(), exit)
    }

    pub(super) fn finish_scope_refs<'a>(
        &self,
        state: &AlternativeState,
        pending: impl ExactSizeIterator<Item = &'a EvidenceRequirements>,
        exit: ScopeExit,
    ) -> Result<ScopeCheckResult, AlternativeError> {
        if state.id != self.id {
            return Err(AlternativeError::PlanMismatch);
        }
        if state.active.len() > self.limits.max_alternatives {
            return Err(AlternativeError::AlternativeLimit);
        }
        let pending_len = pending.len();
        if pending_len > self.limits.max_checks_per_event {
            return Err(AlternativeError::CheckLimit);
        }
        let mut pending_work = 0usize;
        let mut pending_checks = 0usize;
        for request in pending {
            if request.plan != self.id {
                return Err(AlternativeError::PlanMismatch);
            }
            if request.scope != state.scope {
                return Err(AlternativeError::ScopeMismatch);
            }
            if request.state_version != state.version {
                return Err(AlternativeError::StaleTransition);
            }
            if state.last_cursor.is_some_and(|last| request.cursor <= last) {
                return Err(AlternativeError::StaleEvent);
            }
            if request.alternatives.is_empty()
                || request.alternatives.len() > self.limits.max_alternatives
            {
                return Err(AlternativeError::InvalidPendingEvidence);
            }
            let mut seen_alternatives = BTreeSet::new();
            for branch in &request.alternatives {
                if !seen_alternatives.insert(branch.alternative)
                    || !state.active.contains_key(&branch.alternative)
                    || branch.requirements.is_empty()
                {
                    return Err(AlternativeError::InvalidPendingEvidence);
                }
                let alternative = self
                    .alternatives
                    .get(branch.alternative)
                    .ok_or(AlternativeError::InvalidPendingEvidence)?;
                let work = alternative
                    .prior
                    .len()
                    .checked_mul(branch.requirements.len())
                    .ok_or(AlternativeError::ProductWorkLimit)?;
                pending_work = pending_work
                    .checked_add(work)
                    .filter(|work| *work <= self.limits.max_product_work_per_event)
                    .ok_or(AlternativeError::ProductWorkLimit)?;
                let mut seen_requirements = BTreeSet::new();
                for requirement in &branch.requirements {
                    count_check(&mut pending_checks, self.limits.max_checks_per_event)?;
                    if !seen_requirements.insert(*requirement)
                        || !alternative.prior.iter().any(|prior| {
                            prior.guard() == requirement.guard
                                && prior.target() == requirement.target
                        })
                    {
                        return Err(AlternativeError::InvalidPendingEvidence);
                    }
                }
            }
        }
        let mut remaining = self.limits.max_product_work_per_event - pending_work;
        let mut accepting = false;
        for (index, product_state) in &state.active {
            let alternative = self
                .alternatives
                .get(*index)
                .ok_or(AlternativeError::PlanMismatch)?;
            let classification = alternative
                .product
                .classify_state_bounded(product_state, self.limits.max_clauses, remaining)
                .map_err(|error| match error {
                    etas_utils::automaton::StepError::ComponentLimit => {
                        AlternativeError::ClauseLimit
                    }
                    etas_utils::automaton::StepError::WorkLimit => {
                        AlternativeError::ProductWorkLimit
                    }
                    _ => AlternativeError::InvalidProduct,
                })?;
            remaining = remaining
                .checked_sub(classification.work)
                .ok_or(AlternativeError::ProductWorkLimit)?;
            if classification.rejecting {
                return Err(AlternativeError::InvalidProduct);
            }
            accepting |= classification.accepting;
        }
        Ok(if pending_len == 0 && !accepting {
            ScopeCheckResult::Rejected
        } else if exit != ScopeExit::Completed {
            ScopeCheckResult::Incomplete
        } else if pending_len != 0 {
            ScopeCheckResult::PendingEvidence
        } else {
            ScopeCheckResult::Accepting
        })
    }

    pub fn preview<E: ConcretePriorEvidence>(
        &self,
        state: &AlternativeState,
        attempt: &ConcreteActionAttempt<'_>,
        evidence: &E,
    ) -> Result<AlternativeDecision, AlternativeError> {
        if state.id != self.id {
            return Err(AlternativeError::PlanMismatch);
        }
        if state.scope.scope != attempt.scope
            || state.scope.activation != attempt.activation
            || state.scope.revision != attempt.revision
            || state.scope.invocation != attempt.invocation
        {
            return Err(AlternativeError::ScopeMismatch);
        }
        if state.last_cursor.is_some_and(|last| attempt.cursor <= last) {
            return Err(AlternativeError::StaleEvent);
        }
        validate_attempt(attempt, self.limits.evidence).map_err(AlternativeError::Prior)?;
        let mut active = BTreeMap::new();
        let mut pending = Vec::new();
        let mut checks = 0usize;
        let mut product_work = 0usize;
        for (index, product_state) in &state.active {
            let alternative = self
                .alternatives
                .get(*index)
                .ok_or(AlternativeError::PlanMismatch)?;
            let mut allowed = alternative.permissions.is_empty();
            let mut uncertain = false;
            for id in &alternative.permissions {
                count_check(&mut checks, self.limits.max_checks_per_event)?;
                match self.patterns[id]
                    .match_action(attempt.action, self.limits.evidence.pattern)
                    .map_err(AlternativeError::Pattern)?
                {
                    ActionPatternMatch::Matches => allowed = true,
                    ActionPatternMatch::NeedsConcreteSelector => uncertain = true,
                    ActionPatternMatch::Disjoint => {}
                }
            }
            if !allowed {
                if uncertain {
                    return Err(AlternativeError::Prior(
                        PriorEvidenceStepError::UnresolvedSelector,
                    ));
                }
                continue;
            }
            let mut denied = false;
            for id in &alternative.denials {
                count_check(&mut checks, self.limits.max_checks_per_event)?;
                match self.patterns[id]
                    .match_action(attempt.action, self.limits.evidence.pattern)
                    .map_err(AlternativeError::Pattern)?
                {
                    ActionPatternMatch::Matches => denied = true,
                    ActionPatternMatch::NeedsConcreteSelector => {
                        return Err(AlternativeError::Prior(
                            PriorEvidenceStepError::UnresolvedSelector,
                        ));
                    }
                    ActionPatternMatch::Disjoint => {}
                }
            }
            if denied {
                continue;
            }
            for _ in &alternative.prior {
                count_check(&mut checks, self.limits.max_checks_per_event)?;
            }
            let components = alternative.prior.len();
            let work = components
                .checked_mul(
                    components
                        .checked_add(1)
                        .ok_or(AlternativeError::ProductWorkLimit)?,
                )
                .and_then(|value| value.checked_div(2))
                .and_then(|value| {
                    components
                        .checked_mul(2)
                        .and_then(|transitions| value.checked_add(transitions))
                })
                .ok_or(AlternativeError::ProductWorkLimit)?;
            product_work = product_work
                .checked_add(work)
                .ok_or(AlternativeError::ProductWorkLimit)?;
            if product_work > self.limits.max_product_work_per_event {
                return Err(AlternativeError::ProductWorkLimit);
            }
            let matcher = ConcreteMatcher::with_patterns(
                evidence,
                self.limits.evidence.pattern,
                &self.patterns,
            );
            let step = alternative
                .product
                .step_bounded(
                    product_state,
                    attempt,
                    &matcher,
                    ProductStepLimits {
                        max_components: self.limits.max_clauses,
                        max_product_states: 1,
                        max_work: work,
                    },
                )
                .map_err(|_| AlternativeError::InvalidProduct)?;
            if let Some(error) = matcher.issue() {
                return Err(AlternativeError::Prior(error));
            }
            if !step.rejecting.is_empty() {
                continue;
            }
            if matcher.pending() {
                pending.push(PendingAlternativeEvidence {
                    alternative: *index,
                    requirements: matcher.pending_requirements(),
                });
                continue;
            }
            if step.states.len() != 1 || step.accepting != step.states {
                return Err(AlternativeError::InvalidProduct);
            }
            if let Some(next) = step.states.into_iter().next() {
                active.insert(*index, next);
            }
        }
        if !pending.is_empty() {
            return Ok(AlternativeDecision::NeedsEvidence(EvidenceRequirements {
                plan: self.id,
                scope: state.scope,
                state_version: state.version,
                attempt: attempt.id,
                cursor: attempt.cursor,
                alternatives: pending,
            }));
        }
        if active.is_empty() {
            return Ok(AlternativeDecision::Reject);
        }
        Ok(AlternativeDecision::Permit(AlternativeDelta {
            id: self.id,
            scope: state.scope,
            version: state.version,
            cursor: attempt.cursor,
            active,
        }))
    }
}

/// Data-only persisted monitor state of one alternative set. It is untrusted
/// until restored through the compiled alternatives it claims to belong to.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlternativeStateData {
    pub version: u64,
    pub last_cursor: Option<u64>,
    pub active: Vec<AlternativeBranchData>,
}

/// One still-possible alternative and the state of each of its components.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlternativeBranchData {
    pub alternative: u32,
    pub components: Vec<PriorComponentData>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriorComponentData {
    pub component: u32,
    pub rejected: bool,
}

impl AlternativeState {
    /// Exports the state for an engine checkpoint. The data carries no plan
    /// identity; the engine persists that separately and restores through it.
    pub fn to_data(&self) -> AlternativeStateData {
        AlternativeStateData {
            version: self.version,
            last_cursor: self.last_cursor.map(|cursor| cursor.0),
            active: self
                .active
                .iter()
                .map(|(alternative, product)| AlternativeBranchData {
                    alternative: *alternative as u32,
                    components: product
                        .components()
                        .iter()
                        .map(|(component, state)| PriorComponentData {
                            component: *component as u32,
                            rejected: *state == PriorEvidenceState::Rejected,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

impl CompiledTraceAlternatives {
    /// Rebuilds a state from untrusted checkpoint data. Every alternative and
    /// component must exist in this compiled monitor, each restored product
    /// state must be non-rejecting, and at least one alternative must remain.
    pub fn restore_state(
        &self,
        scope: AlternativeScope,
        data: &AlternativeStateData,
    ) -> Result<AlternativeState, AlternativeError> {
        if data.active.is_empty() {
            return Err(AlternativeError::InvalidProduct);
        }
        if data.active.len() > self.limits.max_alternatives {
            return Err(AlternativeError::AlternativeLimit);
        }
        let mut active = BTreeMap::new();
        let mut remaining = self.limits.max_product_work_per_event;
        for branch in &data.active {
            let index = branch.alternative as usize;
            let alternative = self
                .alternatives
                .get(index)
                .ok_or(AlternativeError::InvalidProduct)?;
            if branch.components.len() > self.limits.max_clauses {
                return Err(AlternativeError::ClauseLimit);
            }
            let mut components = BTreeMap::new();
            for component in &branch.components {
                let state = if component.rejected {
                    PriorEvidenceState::Rejected
                } else {
                    PriorEvidenceState::Active
                };
                if components
                    .insert(component.component as usize, state)
                    .is_some()
                {
                    return Err(AlternativeError::InvalidProduct);
                }
            }
            if components.len() != alternative.product.components().len()
                || components
                    .keys()
                    .any(|component| !alternative.product.components().contains_key(component))
            {
                return Err(AlternativeError::InvalidProduct);
            }
            let product = ProductState::new(components);
            let classification = alternative
                .product
                .classify_state_bounded(&product, self.limits.max_clauses, remaining)
                .map_err(|_| AlternativeError::InvalidProduct)?;
            remaining = remaining
                .checked_sub(classification.work)
                .ok_or(AlternativeError::ProductWorkLimit)?;
            if classification.rejecting || !classification.accepting {
                return Err(AlternativeError::InvalidProduct);
            }
            if active.insert(index, product).is_some() {
                return Err(AlternativeError::InvalidProduct);
            }
        }
        Ok(AlternativeState {
            id: self.id,
            scope,
            version: data.version,
            last_cursor: data.last_cursor.map(SemanticEventCursor),
            active,
        })
    }
}

impl AlternativeState {
    /// Applies a preview exactly once, only to the same plan, invocation and version.
    pub fn commit(&mut self, delta: AlternativeDelta) -> Result<(), AlternativeError> {
        let version = self.validate_transition(&delta)?;
        self.apply_validated(delta, version);
        Ok(())
    }

    pub(super) fn validate_transition(
        &self,
        delta: &AlternativeDelta,
    ) -> Result<u64, AlternativeError> {
        if self.id != delta.id {
            return Err(AlternativeError::PlanMismatch);
        }
        if self.scope != delta.scope {
            return Err(AlternativeError::ScopeMismatch);
        }
        if self.version != delta.version {
            return Err(AlternativeError::StaleTransition);
        }
        if self.last_cursor.is_some_and(|last| delta.cursor <= last) {
            return Err(AlternativeError::StaleEvent);
        }
        self.version
            .checked_add(1)
            .ok_or(AlternativeError::VersionExhausted)
    }

    pub(super) fn apply_validated(&mut self, delta: AlternativeDelta, version: u64) {
        self.active = delta.active;
        self.last_cursor = Some(delta.cursor);
        self.version = version;
    }
}

fn count_check(count: &mut usize, max: usize) -> Result<(), AlternativeError> {
    *count = count.checked_add(1).ok_or(AlternativeError::CheckLimit)?;
    if *count > max {
        return Err(AlternativeError::CheckLimit);
    }
    Ok(())
}

fn fingerprint(
    template: &TraceAlternativesTemplate,
    alternatives: &[Alternative],
    limits: AlternativeLimits,
) -> Result<CodeFingerprint, AlternativeError> {
    let mut encoder = CanonicalEncoder::new("etas.code.residual.trace-alternatives.v1");
    encoder.number(u64::from(template.schema_version()));
    encoder.number(u64::from(template.semantic_version()));
    for value in [
        limits.max_alternatives,
        limits.max_clauses,
        limits.max_patterns,
        limits.max_checks_per_event,
        limits.max_product_work_per_event,
        limits.evidence.max_bindings,
        limits.evidence.max_value_bytes,
        limits.evidence.pattern.max_selectors,
        limits.evidence.pattern.max_identity_segments,
        limits.evidence.pattern.max_identity_bytes,
        limits.evidence.pattern.max_literal_bytes,
    ] {
        encoder.number(u64::try_from(value).map_err(|_| AlternativeError::LimitEncoding)?);
    }
    encode_normalized_alternatives(&mut encoder, alternatives)?;
    Ok(encoder.finish())
}

fn semantic_template_id(
    template: &TraceAlternativesTemplate,
    alternatives: &[Alternative],
) -> Result<MonitorTemplateId, AlternativeError> {
    let mut encoder = CanonicalEncoder::new("etas.code.residual.normalized-trace-template.v1");
    encoder.number(u64::from(template.schema_version()));
    encoder.number(u64::from(template.semantic_version()));
    encode_normalized_alternatives(&mut encoder, alternatives)?;
    Ok(MonitorTemplateId::from_canonical_fingerprint(
        encoder.finish(),
    ))
}

fn encode_normalized_alternatives(
    encoder: &mut CanonicalEncoder,
    alternatives: &[Alternative],
) -> Result<(), AlternativeError> {
    encoder.number(u64::try_from(alternatives.len()).map_err(|_| AlternativeError::LimitEncoding)?);
    for alternative in alternatives {
        for ids in [&alternative.permissions, &alternative.denials] {
            encoder.number(u64::try_from(ids.len()).map_err(|_| AlternativeError::LimitEncoding)?);
            for id in ids {
                encoder.fingerprint(id.fingerprint());
            }
        }
        encoder.number(
            u64::try_from(alternative.prior.len()).map_err(|_| AlternativeError::LimitEncoding)?,
        );
        for template in &alternative.prior {
            encoder.fingerprint(template.guard().fingerprint());
            encoder.fingerprint(template.target().fingerprint());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DefinitionKey, DefinitionKind, PackageKey,
        contract::{CanonicalEffect, CanonicalSelector},
        descriptor::EffectTargetInfo,
        residual::{
            monitor::{EvidenceQuery, EvidenceStatus},
            pattern::ActionPatternLimits,
        },
    };

    fn fp(byte: u8) -> CodeFingerprint {
        CodeFingerprint::from_bytes([byte; 32])
    }
    fn limits() -> AlternativeLimits {
        AlternativeLimits {
            evidence: ConcreteEvidenceLimits {
                pattern: ActionPatternLimits {
                    max_selectors: 4,
                    max_identity_segments: 8,
                    max_identity_bytes: 80,
                    max_literal_bytes: 80,
                },
                max_bindings: 8,
                max_value_bytes: 80,
            },
            max_alternatives: 4,
            max_clauses: 16,
            max_patterns: 8,
            max_checks_per_event: 32,
            max_product_work_per_event: 256,
        }
    }
    fn action(name: &str) -> CanonicalEffect {
        CanonicalEffect {
            target: EffectTargetInfo::Action {
                effect: DefinitionKey::new(
                    PackageKey::from_canonical_fingerprint(fp(1)),
                    vec!["app".into()],
                    vec!["Control".into()],
                    DefinitionKind::Effect,
                )
                .unwrap(),
                name: name.into(),
            },
            arguments: vec![],
        }
    }
    fn pattern(name: &str) -> ActionPattern {
        ActionPattern::new(action(name), limits().evidence.pattern).unwrap()
    }

    #[test]
    fn normalized_template_identity_is_stable_and_keeps_complete_alternatives() {
        let guard = pattern("approve");
        let target = pattern("send");
        let rules = vec![
            vec![AlternativeRule::Before {
                guard: guard.clone(),
                target: target.clone(),
            }],
            vec![AlternativeRule::Allow(target.clone())],
        ];
        let compiled = CompiledTraceAlternatives::compile(&rules, limits()).unwrap();
        let mut wider_limits = limits();
        wider_limits.max_checks_per_event += 1;
        let rebuilt = CompiledTraceAlternatives::from_template(
            compiled.template(),
            &compiled.patterns,
            wider_limits,
        )
        .unwrap();
        assert_eq!(compiled.template_id(), rebuilt.template_id());
        assert_ne!(
            compiled.id, rebuilt.id,
            "execution limits bind runtime state"
        );

        let conjunction = CompiledTraceAlternatives::compile(
            &[vec![
                AlternativeRule::Before {
                    guard: guard.clone(),
                    target: target.clone(),
                },
                AlternativeRule::Allow(target.clone()),
            ]],
            limits(),
        )
        .unwrap();
        assert_ne!(compiled.template_id(), conjunction.template_id());
        let reversed = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Before {
                guard: target,
                target: guard,
            }]],
            limits(),
        )
        .unwrap();
        let forward = CompiledTraceAlternatives::compile(&[rules[0].clone()], limits()).unwrap();
        let redundant_permission = CompiledTraceAlternatives::compile(
            &[vec![
                rules[0][0].clone(),
                AlternativeRule::Allow(pattern("send")),
            ]],
            limits(),
        )
        .unwrap();
        assert_eq!(forward.template_id(), redundant_permission.template_id());
        assert_ne!(forward.template_id(), reversed.template_id());
    }
    fn scope() -> AlternativeScope {
        AlternativeScope {
            scope: SemanticScopeId::from_canonical_fingerprint(fp(2)),
            activation: ActivationSiteId::from_canonical_fingerprint(fp(3)),
            revision: RevisionKey::from_canonical_fingerprint(fp(4)),
            invocation: ScopeInvocationId::from_bytes([5; 16]),
        }
    }
    #[derive(Clone, Copy)]
    enum TestStatus {
        Accepted,
        Absent,
        Denied,
        Pending,
    }

    fn status(status: TestStatus, query: EvidenceQuery<'_, '_>) -> EvidenceStatus {
        match status {
            TestStatus::Accepted => EvidenceStatus::Accepted(super::super::AcceptedPriorEvidence {
                scope: query.attempt.scope,
                activation: query.attempt.activation,
                revision: query.attempt.revision,
                invocation: query.attempt.invocation,
                guard_pattern: query.guard.id(),
                target_pattern: query.target.id(),
                guard_attempt: super::super::ActionAttemptId::from_bytes([2; 16]),
                completed_at: super::super::SemanticEventCursor(2),
            }),
            TestStatus::Absent => EvidenceStatus::Absent,
            TestStatus::Denied => EvidenceStatus::Denied,
            TestStatus::Pending => EvidenceStatus::Pending,
        }
    }

    struct Provider(TestStatus);
    impl ConcretePriorEvidence for Provider {
        fn query(&self, query: EvidenceQuery<'_, '_>) -> EvidenceStatus {
            status(self.0, query)
        }
    }

    struct PerGuard {
        approve: TestStatus,
        review: TestStatus,
    }

    impl ConcretePriorEvidence for PerGuard {
        fn query(&self, query: EvidenceQuery<'_, '_>) -> EvidenceStatus {
            let selected = match &query.guard.effect().target {
                EffectTargetInfo::Action { name, .. } if name == "approve" => self.approve,
                EffectTargetInfo::Action { name, .. } if name == "review" => self.review,
                _ => return EvidenceStatus::Unavailable,
            };
            status(selected, query)
        }
    }
    fn attempt<'a>(
        action: &'a CanonicalEffect,
        bindings: &'a BTreeMap<
            crate::residual::identity::ValueBindingId,
            crate::residual::predicate::BorrowedValue<'a>,
        >,
    ) -> ConcreteActionAttempt<'a> {
        use crate::residual::binding::{BindingLayout, BindingLayoutLimits, BorrowedBindingLimits};
        static EMPTY_LAYOUT: std::sync::OnceLock<BindingLayout> = std::sync::OnceLock::new();
        let layout = EMPTY_LAYOUT.get_or_init(|| {
            BindingLayout::new(
                BTreeMap::new(),
                BindingLayoutLimits {
                    max_bindings: 0,
                    max_projections_per_binding: 0,
                    max_total_projections: 0,
                },
            )
            .unwrap()
        });
        let scope = scope();
        let cursor = match &action.target {
            EffectTargetInfo::Action { name, .. } if name == "approve" => 2,
            _ => 3,
        };
        ConcreteActionAttempt {
            scope: scope.scope,
            activation: scope.activation,
            revision: scope.revision,
            invocation: scope.invocation,
            id: super::super::ActionAttemptId::from_bytes([cursor as u8; 16]),
            cursor: super::super::SemanticEventCursor(cursor),
            action,
            bindings: layout
                .validate_borrowed(
                    bindings,
                    BorrowedBindingLimits {
                        max_bindings: 8,
                        max_value_bytes: 1024,
                        max_total_value_bytes: 8192,
                    },
                )
                .unwrap(),
        }
    }
    fn permit(decision: AlternativeDecision) -> AlternativeDelta {
        match decision {
            AlternativeDecision::Permit(delta) => delta,
            _ => panic!("expected permit"),
        }
    }

    #[test]
    fn denied_effect_family_rejects_inherited_action_and_permits_unrelated_action() {
        let EffectTargetInfo::Action { effect: root, .. } = action("read").target else {
            unreachable!()
        };
        let child = DefinitionKey::new(
            root.package(),
            vec!["app".into()],
            vec!["Child".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        let other = DefinitionKey::new(
            root.package(),
            vec!["app".into()],
            vec!["Other".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        let family = ActionPattern::from_effect_family(
            CanonicalEffect {
                target: EffectTargetInfo::Effect(root),
                arguments: vec![],
            },
            vec![child.clone()],
            limits().evidence.pattern,
        )
        .unwrap();
        let monitor =
            CompiledTraceAlternatives::compile(&[vec![AlternativeRule::Deny(family)]], limits())
                .unwrap();
        let state = monitor.initial_state(scope());
        let child_action = CanonicalEffect {
            target: EffectTargetInfo::Action {
                effect: child,
                name: "read".into(),
            },
            arguments: vec![],
        };
        let other_action = CanonicalEffect {
            target: EffectTargetInfo::Action {
                effect: other,
                name: "read".into(),
            },
            arguments: vec![],
        };
        let bindings = BTreeMap::new();
        assert!(matches!(
            monitor
                .preview(
                    &state,
                    &attempt(&child_action, &bindings),
                    &Provider(TestStatus::Absent)
                )
                .unwrap(),
            AlternativeDecision::Reject
        ));
        assert!(matches!(
            monitor
                .preview(
                    &state,
                    &attempt(&other_action, &bindings),
                    &Provider(TestStatus::Absent)
                )
                .unwrap(),
            AlternativeDecision::Permit(_)
        ));
    }

    #[test]
    fn deny_does_not_permit_an_unresolved_type_selector_comparison() {
        let mut declared = action("read");
        declared.arguments = vec![CanonicalSelector::Type(fp(10))];
        let pattern = ActionPattern::new(declared.clone(), limits().evidence.pattern).unwrap();
        let monitor =
            CompiledTraceAlternatives::compile(&[vec![AlternativeRule::Deny(pattern)]], limits())
                .unwrap();
        let state = monitor.initial_state(scope());
        let bindings = BTreeMap::new();
        declared.arguments = vec![CanonicalSelector::Type(fp(11))];
        assert!(matches!(
            monitor.preview(
                &state,
                &attempt(&declared, &bindings),
                &Provider(TestStatus::Absent)
            ),
            Err(AlternativeError::Prior(
                PriorEvidenceStepError::UnresolvedSelector
            ))
        ));
    }

    #[test]
    fn prior_template_roundtrip_rebuilds_only_from_checked_patterns() {
        let guard = pattern("approve");
        let target = pattern("send");
        let template = PriorEvidenceTemplate::new(guard.id(), target.id());
        let encoded = serde_json::to_value(template).unwrap();
        let decoded: PriorEvidenceTemplate = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, template);
        let mut patterns =
            BTreeMap::from([(guard.id(), guard.clone()), (target.id(), target.clone())]);
        assert!(
            decoded
                .rebuild(&patterns, limits().evidence.pattern)
                .is_ok()
        );
        patterns.remove(&target.id());
        assert!(matches!(
            decoded.rebuild(&patterns, limits().evidence.pattern),
            Err(PriorEvidenceTemplateError::MissingPattern(id)) if id == target.id()
        ));
        patterns.insert(target.id(), guard.clone());
        assert!(matches!(
            decoded.rebuild(&patterns, limits().evidence.pattern),
            Err(PriorEvidenceTemplateError::PatternIdentityMismatch(id)) if id == target.id()
        ));
        patterns.insert(target.id(), target);
        let mut limited = limits().evidence.pattern;
        limited.max_identity_bytes = 0;
        assert!(matches!(
            decoded.rebuild(&patterns, limited),
            Err(PriorEvidenceTemplateError::InvalidPattern(_))
        ));
        for (field, expected) in [("schema_version", 0), ("semantic_version", 1)] {
            let mut invalid = encoded.clone();
            invalid[field] = serde_json::json!(99);
            let invalid: PriorEvidenceTemplate = serde_json::from_value(invalid).unwrap();
            assert!(matches!(
                (
                    expected,
                    invalid.rebuild(&patterns, limits().evidence.pattern)
                ),
                (0, Err(PriorEvidenceTemplateError::UnsupportedSchema(99)))
                    | (1, Err(PriorEvidenceTemplateError::UnsupportedSemantics(99)))
            ));
        }
        let mut extra = encoded;
        extra["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<PriorEvidenceTemplate>(extra).is_err());
    }

    #[test]
    fn alternative_template_roundtrip_preserves_or_branches_and_rejects_tampering() {
        let approve = pattern("approve");
        let send = pattern("send");
        let audit = pattern("audit");
        let original = CompiledTraceAlternatives::compile(
            &[
                vec![AlternativeRule::Before {
                    guard: approve.clone(),
                    target: send.clone(),
                }],
                vec![
                    AlternativeRule::Allow(send.clone()),
                    AlternativeRule::Allow(audit.clone()),
                ],
            ],
            limits(),
        )
        .unwrap();
        let patterns = [approve.clone(), send.clone(), audit]
            .into_iter()
            .map(|p| (p.id(), p))
            .collect();
        let encoded = serde_json::to_value(original.template()).unwrap();
        let template: TraceAlternativesTemplate = serde_json::from_value(encoded.clone()).unwrap();
        let rebuilt =
            CompiledTraceAlternatives::from_template(&template, &patterns, limits()).unwrap();
        assert_eq!(original.id, rebuilt.id);
        let values = BTreeMap::new();
        let guard_action = action("approve");
        let target_action = action("send");
        for monitor in [&original, &rebuilt] {
            let mut state = monitor.initial_state(scope());
            let step = monitor
                .preview(
                    &state,
                    &attempt(&guard_action, &values),
                    &Provider(TestStatus::Absent),
                )
                .unwrap();
            state.commit(permit(step)).unwrap();
            assert!(matches!(
                monitor.preview(
                    &state,
                    &attempt(&target_action, &values),
                    &Provider(TestStatus::Denied)
                ),
                Ok(AlternativeDecision::Reject)
            ));
        }
        let mut missing = patterns.clone();
        missing.remove(&approve.id());
        assert!(
            matches!(CompiledTraceAlternatives::from_template(&template, &missing, limits()), Err(AlternativeError::Template(PriorEvidenceTemplateError::MissingPattern(id))) if id == approve.id())
        );
        let mut changed = encoded.clone();
        changed["semantic_version"] = serde_json::json!(2);
        let changed: TraceAlternativesTemplate = serde_json::from_value(changed).unwrap();
        assert!(matches!(
            CompiledTraceAlternatives::from_template(&changed, &patterns, limits()),
            Err(AlternativeError::Template(
                PriorEvidenceTemplateError::UnsupportedSemantics(2)
            ))
        ));
        let mut changed = encoded;
        changed["alternatives"][0][0]["Before"]["schema_version"] = serde_json::json!(2);
        let changed: TraceAlternativesTemplate = serde_json::from_value(changed).unwrap();
        assert!(matches!(
            CompiledTraceAlternatives::from_template(&changed, &patterns, limits()),
            Err(AlternativeError::Template(
                PriorEvidenceTemplateError::UnsupportedSchema(2)
            ))
        ));
        let mut restricted = limits();
        restricted.max_patterns = 1;
        assert!(matches!(
            CompiledTraceAlternatives::from_template(&template, &patterns, restricted),
            Err(AlternativeError::PatternLimit)
        ));
    }

    #[test]
    fn alternative_template_decode_uses_a_shared_node_budget() {
        let mut encoded = serde_json::Map::new();
        encoded.insert("schema_version".into(), serde_json::json!(1));
        encoded.insert("semantic_version".into(), serde_json::json!(1));
        encoded.insert(
            "alternatives".into(),
            serde_json::Value::Array(vec![serde_json::json!([]); MAX_TRACE_TEMPLATE_NODES + 1]),
        );
        assert!(serde_json::from_value::<TraceAlternativesTemplate>(encoded.into()).is_err());

        let clause =
            serde_json::to_value(TraceAlternativeClause::Allow(pattern("send").id())).unwrap();
        let half = MAX_TRACE_TEMPLATE_NODES / 2;
        let input = |count: usize| {
            serde_json::json!({
                "schema_version": 1,
                "semantic_version": 1,
                "alternatives": [vec![clause.clone(); count], vec![clause.clone(); count]],
            })
        };
        assert!(serde_json::from_value::<TraceAlternativesTemplate>(input(half - 1)).is_ok());
        assert!(serde_json::from_value::<TraceAlternativesTemplate>(input(half)).is_err());
    }

    #[test]
    fn repeated_prior_clauses_share_checked_patterns_after_rebuild() {
        let approve = pattern("approve");
        let send = pattern("send");
        let mut bounds = limits();
        bounds.max_clauses = 2048;
        bounds.max_patterns = 2;
        let rules = vec![vec![
            AlternativeRule::Before {
                guard: approve.clone(),
                target: send.clone(),
            };
            2048
        ]];
        let original = CompiledTraceAlternatives::compile(&rules, bounds).unwrap();
        let patterns = BTreeMap::from([(approve.id(), approve), (send.id(), send)]);
        let restored =
            CompiledTraceAlternatives::from_template(original.template(), &patterns, bounds)
                .unwrap();
        assert_eq!(restored.patterns.len(), 2);
        assert_eq!(restored.alternatives[0].prior.len(), 2048);
        assert_eq!(restored.id, original.id);
    }

    #[test]
    fn alternatives_keep_permissions_and_evidence_together_across_actions() {
        let monitor = CompiledTraceAlternatives::compile(
            &[
                vec![AlternativeRule::Before {
                    guard: pattern("approve"),
                    target: pattern("send"),
                }],
                vec![
                    AlternativeRule::Allow(pattern("send")),
                    AlternativeRule::Allow(pattern("audit")),
                ],
            ],
            limits(),
        )
        .unwrap();
        let mut state = monitor.initial_state(scope());
        assert_eq!(
            monitor.finish_scope(&state, &[], ScopeExit::Completed),
            Ok(ScopeCheckResult::Accepting)
        );
        let values = BTreeMap::new();
        let approve = action("approve");
        let send = action("send");
        let first = monitor
            .preview(
                &state,
                &attempt(&approve, &values),
                &Provider(TestStatus::Absent),
            )
            .unwrap();
        state.commit(permit(first)).unwrap();
        assert!(matches!(
            monitor.preview(
                &state,
                &attempt(&send, &values),
                &Provider(TestStatus::Denied)
            ),
            Ok(AlternativeDecision::Reject)
        ));
        let accepted = monitor
            .preview(
                &state,
                &attempt(&send, &values),
                &Provider(TestStatus::Accepted),
            )
            .unwrap();
        state.commit(permit(accepted)).unwrap();
        assert_eq!(
            monitor.finish_scope(&state, &[], ScopeExit::Completed),
            Ok(ScopeCheckResult::Accepting)
        );
    }

    #[test]
    fn pending_alternative_is_not_dropped_even_if_another_allows() {
        let monitor = CompiledTraceAlternatives::compile(
            &[
                vec![AlternativeRule::Before {
                    guard: pattern("approve"),
                    target: pattern("send"),
                }],
                vec![
                    AlternativeRule::Allow(pattern("send")),
                    AlternativeRule::Allow(pattern("audit")),
                ],
            ],
            limits(),
        )
        .unwrap();
        let state = monitor.initial_state(scope());
        let values = BTreeMap::new();
        let send = action("send");
        let pending = match monitor
            .preview(
                &state,
                &attempt(&send, &values),
                &Provider(TestStatus::Pending),
            )
            .unwrap()
        {
            AlternativeDecision::NeedsEvidence(pending) => pending,
            _ => panic!("expected pending evidence"),
        };
        assert_eq!(
            monitor.finish_scope(&state, &[pending], ScopeExit::Completed),
            Ok(ScopeCheckResult::PendingEvidence)
        );
        assert!(matches!(
            monitor.preview(
                &state,
                &attempt(&send, &values),
                &Provider(TestStatus::Accepted)
            ),
            Ok(AlternativeDecision::Permit(_))
        ));
    }

    #[test]
    fn pending_requirements_preserve_branches_and_event_identity() {
        let approve = pattern("approve");
        let review = pattern("review");
        let target = pattern("send");
        let monitor = CompiledTraceAlternatives::compile(
            &[
                vec![
                    AlternativeRule::Before {
                        guard: approve.clone(),
                        target: target.clone(),
                    },
                    AlternativeRule::Before {
                        guard: review.clone(),
                        target: target.clone(),
                    },
                ],
                vec![AlternativeRule::Before {
                    guard: review.clone(),
                    target: target.clone(),
                }],
                vec![AlternativeRule::Allow(target.clone())],
            ],
            limits(),
        )
        .unwrap();
        let mut state = monitor.initial_state(scope());
        let values = BTreeMap::new();
        let send = action("send");
        let event = attempt(&send, &values);
        let AlternativeDecision::NeedsEvidence(requirements) = monitor
            .preview(&state, &event, &Provider(TestStatus::Pending))
            .unwrap()
        else {
            panic!("pending evidence must not be a permit")
        };
        assert_eq!(requirements.plan, monitor.id);
        assert_eq!(requirements.scope, scope());
        assert_eq!(requirements.state_version, 0);
        assert_eq!(requirements.attempt, event.id);
        assert_eq!(requirements.cursor, event.cursor);
        assert_eq!(requirements.alternatives.len(), 2);
        assert_eq!(requirements.alternatives[0].alternative, 0);
        assert_eq!(requirements.alternatives[1].alternative, 1);
        assert_eq!(
            monitor.finish_scope(
                &state,
                std::slice::from_ref(&requirements),
                ScopeExit::Completed
            ),
            Ok(ScopeCheckResult::PendingEvidence)
        );
        for exit in [ScopeExit::Failed, ScopeExit::Cancelled] {
            assert_eq!(
                monitor.finish_scope(&state, std::slice::from_ref(&requirements), exit),
                Ok(ScopeCheckResult::Incomplete)
            );
        }
        let mut invalid = requirements.clone();
        invalid.alternatives[0].alternative = usize::MAX;
        assert_eq!(
            monitor.finish_scope(&state, &[invalid], ScopeExit::Completed),
            Err(AlternativeError::InvalidPendingEvidence)
        );
        let mut invalid = requirements.clone();
        invalid.alternatives[0].requirements[0].guard = pattern("unrelated").id();
        assert_eq!(
            monitor.finish_scope(&state, &[invalid], ScopeExit::Completed),
            Err(AlternativeError::InvalidPendingEvidence)
        );
        let mut invalid = requirements.clone();
        invalid.alternatives.push(invalid.alternatives[0].clone());
        assert_eq!(
            monitor.finish_scope(&state, &[invalid], ScopeExit::Completed),
            Err(AlternativeError::InvalidPendingEvidence)
        );
        let first = &requirements.alternatives[0].requirements;
        assert_eq!(first.len(), 2);
        assert!(first.contains(&PriorEvidenceRequirement {
            guard: approve.id(),
            target: target.id()
        }));
        assert!(first.contains(&PriorEvidenceRequirement {
            guard: review.id(),
            target: target.id()
        }));
        assert_eq!(
            requirements.alternatives[1].requirements,
            vec![PriorEvidenceRequirement {
                guard: review.id(),
                target: target.id()
            }]
        );
        let AlternativeDecision::NeedsEvidence(narrowed) = monitor
            .preview(
                &state,
                &event,
                &PerGuard {
                    approve: TestStatus::Pending,
                    review: TestStatus::Accepted,
                },
            )
            .unwrap()
        else {
            panic!("the first branch still needs evidence")
        };
        assert_eq!(narrowed.alternatives.len(), 1);
        assert_eq!(narrowed.alternatives[0].alternative, 0);
        assert_eq!(
            narrowed.alternatives[0].requirements,
            vec![PriorEvidenceRequirement {
                guard: approve.id(),
                target: target.id(),
            }]
        );
        assert!(matches!(
            monitor.preview(
                &state,
                &event,
                &PerGuard {
                    approve: TestStatus::Pending,
                    review: TestStatus::Denied,
                },
            ),
            Ok(AlternativeDecision::Permit(_))
        ));
        let decision = monitor
            .preview(&state, &event, &Provider(TestStatus::Accepted))
            .unwrap();
        state.commit(permit(decision)).unwrap();
        assert_eq!(
            monitor.finish_scope(&state, &[requirements], ScopeExit::Completed),
            Err(AlternativeError::StaleTransition)
        );
        assert!(matches!(
            monitor.preview(&state, &event, &Provider(TestStatus::Accepted)),
            Err(AlternativeError::StaleEvent)
        ));
    }

    #[test]
    fn scope_exit_validates_the_plan_and_every_active_product() {
        let rules = [vec![AlternativeRule::Before {
            guard: pattern("approve"),
            target: pattern("send"),
        }]];
        let monitor = CompiledTraceAlternatives::compile(&rules, limits()).unwrap();
        let other = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Allow(pattern("audit"))]],
            limits(),
        )
        .unwrap();
        assert_eq!(
            monitor.finish_scope(&other.initial_state(scope()), &[], ScopeExit::Completed),
            Err(AlternativeError::PlanMismatch)
        );
        let mut state = monitor.initial_state(scope());
        for exit in [ScopeExit::Failed, ScopeExit::Cancelled] {
            assert_eq!(
                monitor.finish_scope(&state, &[], exit),
                Ok(ScopeCheckResult::Incomplete)
            );
        }
        state.active.insert(
            0,
            ProductState::new(BTreeMap::from([(0, PriorEvidenceState::Rejected)])),
        );
        assert_eq!(
            monitor.finish_scope(&state, &[], ScopeExit::Completed),
            Err(AlternativeError::InvalidProduct)
        );
        state.active.insert(0, ProductState::new(BTreeMap::new()));
        assert_eq!(
            monitor.finish_scope(&state, &[], ScopeExit::Completed),
            Err(AlternativeError::InvalidProduct)
        );
        state.active.clear();
        assert_eq!(
            monitor.finish_scope(&state, &[], ScopeExit::Completed),
            Ok(ScopeCheckResult::Rejected)
        );
        assert_eq!(
            monitor.finish_scope(&state, &[], ScopeExit::Cancelled),
            Ok(ScopeCheckResult::Rejected)
        );
        let mut constrained = limits();
        constrained.max_product_work_per_event = 2;
        let constrained = CompiledTraceAlternatives::compile(&rules, constrained).unwrap();
        assert_eq!(
            constrained.finish_scope(
                &constrained.initial_state(scope()),
                &[],
                ScopeExit::Completed
            ),
            Err(AlternativeError::ProductWorkLimit)
        );
    }

    #[test]
    fn product_requires_all_prior_evidence_and_bounds_combined_work() {
        let rules = [vec![
            AlternativeRule::Before {
                guard: pattern("approve"),
                target: pattern("send"),
            },
            AlternativeRule::Before {
                guard: pattern("review"),
                target: pattern("send"),
            },
        ]];
        let monitor = CompiledTraceAlternatives::compile(&rules, limits()).unwrap();
        let state = monitor.initial_state(scope());
        let values = BTreeMap::new();
        let send = action("send");
        for (approve, review, expected) in [
            (TestStatus::Accepted, TestStatus::Accepted, 0),
            (TestStatus::Accepted, TestStatus::Pending, 1),
            (TestStatus::Pending, TestStatus::Denied, 2),
        ] {
            let result = monitor
                .preview(
                    &state,
                    &attempt(&send, &values),
                    &PerGuard { approve, review },
                )
                .unwrap();
            assert!(matches!(
                (expected, result),
                (0, AlternativeDecision::Permit(_))
                    | (1, AlternativeDecision::NeedsEvidence(_))
                    | (2, AlternativeDecision::Reject)
            ));
        }
        let mut bound = limits();
        // Two single-transition components require seven product work units.
        bound.max_product_work_per_event = 6;
        let limited = CompiledTraceAlternatives::compile(&rules, bound).unwrap();
        assert!(matches!(
            limited.preview(
                &limited.initial_state(scope()),
                &attempt(&send, &values),
                &PerGuard {
                    approve: TestStatus::Accepted,
                    review: TestStatus::Accepted
                }
            ),
            Err(AlternativeError::ProductWorkLimit)
        ));
    }

    #[test]
    fn deny_overrides_implicit_permission_and_deny_only_does_not_limit_other_actions() {
        let monitor = CompiledTraceAlternatives::compile(
            &[vec![
                AlternativeRule::Before {
                    guard: pattern("approve"),
                    target: pattern("send"),
                },
                AlternativeRule::Deny(pattern("send")),
            ]],
            limits(),
        )
        .unwrap();
        let state = monitor.initial_state(scope());
        let values = BTreeMap::new();
        let send = action("send");
        assert!(matches!(
            monitor.preview(
                &state,
                &attempt(&send, &values),
                &Provider(TestStatus::Accepted)
            ),
            Ok(AlternativeDecision::Reject)
        ));
        let deny_only = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Deny(pattern("send"))]],
            limits(),
        )
        .unwrap();
        let other = action("audit");
        assert!(matches!(
            deny_only.preview(
                &deny_only.initial_state(scope()),
                &attempt(&other, &values),
                &Provider(TestStatus::Absent)
            ),
            Ok(AlternativeDecision::Permit(_))
        ));
    }

    #[test]
    fn deltas_are_versioned_and_bound_to_scope_and_plan() {
        let monitor = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Allow(pattern("send"))]],
            limits(),
        )
        .unwrap();
        let mut state = monitor.initial_state(scope());
        let values = BTreeMap::new();
        let send = action("send");
        let first = permit(
            monitor
                .preview(
                    &state,
                    &attempt(&send, &values),
                    &Provider(TestStatus::Absent),
                )
                .unwrap(),
        );
        let stale = permit(
            monitor
                .preview(
                    &state,
                    &attempt(&send, &values),
                    &Provider(TestStatus::Absent),
                )
                .unwrap(),
        );
        state.commit(first).unwrap();
        assert_eq!(state.commit(stale), Err(AlternativeError::StaleTransition));
        assert!(matches!(
            monitor.preview(
                &state,
                &attempt(&send, &values),
                &Provider(TestStatus::Absent)
            ),
            Err(AlternativeError::StaleEvent)
        ));
        let mut older = attempt(&send, &values);
        older.cursor = super::super::SemanticEventCursor(2);
        assert!(matches!(
            monitor.preview(&state, &older, &Provider(TestStatus::Absent)),
            Err(AlternativeError::StaleEvent)
        ));
        let mut next = attempt(&send, &values);
        next.id = super::super::ActionAttemptId::from_bytes([4; 16]);
        next.cursor = super::super::SemanticEventCursor(4);
        let delta = permit(
            monitor
                .preview(&state, &next, &Provider(TestStatus::Absent))
                .unwrap(),
        );
        let mut other_scope = scope();
        other_scope.invocation = ScopeInvocationId::from_bytes([9; 16]);
        assert_eq!(
            monitor.initial_state(other_scope).commit(delta),
            Err(AlternativeError::ScopeMismatch)
        );
        let other = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Allow(pattern("audit"))]],
            limits(),
        )
        .unwrap();
        assert!(matches!(
            other.preview(
                &state,
                &attempt(&send, &values),
                &Provider(TestStatus::Absent)
            ),
            Err(AlternativeError::PlanMismatch)
        ));
        let mut wrong_attempt = attempt(&send, &values);
        wrong_attempt.invocation = other_scope.invocation;
        assert!(matches!(
            monitor.preview(&state, &wrong_attempt, &Provider(TestStatus::Absent)),
            Err(AlternativeError::ScopeMismatch)
        ));
    }

    #[test]
    fn construction_and_event_limits_fail_closed() {
        assert!(matches!(
            CompiledTraceAlternatives::compile(&[], limits()),
            Err(AlternativeError::EmptyAlternatives)
        ));
        assert!(matches!(
            CompiledTraceAlternatives::compile(&[vec![]], limits()),
            Err(AlternativeError::EmptyAlternative)
        ));
        let mut bound = limits();
        bound.max_patterns = 0;
        assert!(matches!(
            CompiledTraceAlternatives::compile(
                &[vec![AlternativeRule::Allow(pattern("send"))]],
                bound
            ),
            Err(AlternativeError::PatternLimit)
        ));
        bound = limits();
        bound.max_checks_per_event = 0;
        let monitor = CompiledTraceAlternatives::compile(
            &[vec![AlternativeRule::Allow(pattern("send"))]],
            bound,
        )
        .unwrap();
        let values = BTreeMap::new();
        let send = action("send");
        assert!(matches!(
            monitor.preview(
                &monitor.initial_state(scope()),
                &attempt(&send, &values),
                &Provider(TestStatus::Absent)
            ),
            Err(AlternativeError::CheckLimit)
        ));
    }
}
