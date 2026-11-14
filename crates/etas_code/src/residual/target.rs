//! Neutral backend capability descriptions for residual monitor plans.
//!
//! Capabilities describe implemented engine semantics: plan schemas, event
//! phases, evidence kinds, scope exits and resource ceilings. They are not
//! credentials, live grants, configured endpoints or approval of any action,
//! and a support result is not installation evidence. Checking support never
//! opens a service or executes a candidate.

use std::collections::BTreeSet;

use crate::{CodeFingerprint, identity::canonical::CanonicalEncoder};

use super::{
    identity::PlanHash,
    monitor::{SymbolicTraceRule, TraceAlternativeClause, TraceComponentMonitor},
    plan::{
        PLAN_SCHEMA_VERSION, PLAN_SEMANTIC_VERSION, SemanticPlanComponent, SemanticResidualPlan,
    },
};

const TARGET_SCHEMA_VERSION: u32 = 1;

/// Plan component families an engine can execute.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    etas_core::RetainedSize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ResidualComponentKind {
    ActionPredicate,
    TraceAlternatives,
    GenericTraceAlternatives,
}

/// Normalized semantic event phases a governed action passes through.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    etas_core::RetainedSize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ResidualEventPhase {
    /// Checked before dispatch, for every source/default/forwarded/std/agent
    /// and tool path, before any governed side effect.
    PreDispatchAttempt,
    /// A correlated completed outcome of a dispatched action.
    CorrelatedOutcome,
}

/// Evidence the engine can supply to the shared monitor kernel.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    etas_core::RetainedSize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ResidualEvidenceKind {
    /// A provider-authenticated accepted outcome of a prior guard action,
    /// correlated by scope, invocation, attempt and ordering.
    AcceptedPriorOutcome,
}

/// Terminal paths at which an active scope's monitor is checked.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    etas_core::RetainedSize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ResidualScopeExitKind {
    Return,
    NonLocalReturn,
    ErrorPropagation,
    HandlerFinish,
    Abort,
    Cancellation,
}

impl ResidualScopeExitKind {
    pub const ALL: [Self; 6] = [
        Self::Return,
        Self::NonLocalReturn,
        Self::ErrorPropagation,
        Self::HandlerFinish,
        Self::Abort,
        Self::Cancellation,
    ];
}

/// Monitor-state persistence across suspension, which is never scope exit.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    etas_core::RetainedSize,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ResidualSuspensionKind {
    HostAwait,
    CheckpointRestore,
    ConcurrentChildren,
}

impl ResidualSuspensionKind {
    pub const ALL: [Self; 3] = [
        Self::HostAwait,
        Self::CheckpointRestore,
        Self::ConcurrentChildren,
    ];
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, etas_core::RetainedSize, serde::Serialize, serde::Deserialize,
)]
pub struct ResidualTargetLimits {
    pub max_obligations_per_plan: usize,
    pub max_alternatives: usize,
    pub max_clauses: usize,
    pub max_patterns: usize,
}

impl ResidualTargetLimits {
    pub const NONE: Self = Self {
        max_obligations_per_plan: 0,
        max_alternatives: 0,
        max_clauses: 0,
        max_patterns: 0,
    };
}

/// Implemented residual semantics of one execution target.
#[derive(
    Clone, Debug, PartialEq, Eq, etas_core::RetainedSize, serde::Serialize, serde::Deserialize,
)]
pub struct ResidualTargetCapabilities {
    /// Stable identity and semantic version of the engine's residual kernel.
    pub engine: String,
    pub engine_semantic_version: u32,
    /// Supported `(schema, semantic)` versions of semantic plans.
    pub plan_versions: BTreeSet<(u32, u32)>,
    /// Supported `(schema, semantic)` versions of trace templates.
    pub trace_template_versions: BTreeSet<(u32, u32)>,
    pub components: BTreeSet<ResidualComponentKind>,
    pub phases: BTreeSet<ResidualEventPhase>,
    pub evidence: BTreeSet<ResidualEvidenceKind>,
    pub exits: BTreeSet<ResidualScopeExitKind>,
    pub suspension: BTreeSet<ResidualSuspensionKind>,
    pub limits: ResidualTargetLimits,
}

impl Default for ResidualTargetCapabilities {
    fn default() -> Self {
        Self::none()
    }
}

impl ResidualTargetCapabilities {
    /// A target with no residual enforcement. Every residual plan is
    /// unsupported; this is the default for analysis-only consumers.
    pub fn none() -> Self {
        Self {
            engine: String::new(),
            engine_semantic_version: 0,
            plan_versions: BTreeSet::new(),
            trace_template_versions: BTreeSet::new(),
            components: BTreeSet::new(),
            phases: BTreeSet::new(),
            evidence: BTreeSet::new(),
            exits: BTreeSet::new(),
            suspension: BTreeSet::new(),
            limits: ResidualTargetLimits::NONE,
        }
    }

    pub fn is_none(&self) -> bool {
        *self == Self::none()
    }

    /// Canonical identity for cache/session inputs and support results.
    pub fn fingerprint(&self) -> CodeFingerprint {
        let mut encoder = CanonicalEncoder::new("etas.code.residual.target-capabilities.v1");
        encoder.number(u64::from(TARGET_SCHEMA_VERSION));
        encoder.bytes(self.engine.as_bytes());
        encoder.number(u64::from(self.engine_semantic_version));
        for set in [&self.plan_versions, &self.trace_template_versions] {
            encoder.number(set.len() as u64);
            for (schema, semantic) in set {
                encoder.number(u64::from(*schema));
                encoder.number(u64::from(*semantic));
            }
        }
        encoder.number(self.components.len() as u64);
        for value in &self.components {
            encoder.tag(*value as u8);
        }
        encoder.number(self.phases.len() as u64);
        for value in &self.phases {
            encoder.tag(*value as u8);
        }
        encoder.number(self.evidence.len() as u64);
        for value in &self.evidence {
            encoder.tag(*value as u8);
        }
        encoder.number(self.exits.len() as u64);
        for value in &self.exits {
            encoder.tag(*value as u8);
        }
        encoder.number(self.suspension.len() as u64);
        for value in &self.suspension {
            encoder.tag(*value as u8);
        }
        for limit in [
            self.limits.max_obligations_per_plan,
            self.limits.max_alternatives,
            self.limits.max_clauses,
            self.limits.max_patterns,
        ] {
            encoder.number(limit as u64);
        }
        encoder.finish()
    }
}

/// Semantics a plan needs from its target, derived from plan contents only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResidualPlanRequirements {
    pub plan_version: (u32, u32),
    pub trace_template_versions: BTreeSet<(u32, u32)>,
    pub components: BTreeSet<ResidualComponentKind>,
    pub phases: BTreeSet<ResidualEventPhase>,
    pub evidence: BTreeSet<ResidualEvidenceKind>,
    pub exits: BTreeSet<ResidualScopeExitKind>,
    pub suspension: BTreeSet<ResidualSuspensionKind>,
    pub obligations: usize,
    pub max_alternatives: usize,
    pub max_clauses: usize,
    pub max_patterns: usize,
}

impl ResidualPlanRequirements {
    pub fn of(plan: &SemanticResidualPlan) -> Self {
        let mut requirements = Self {
            plan_version: (PLAN_SCHEMA_VERSION, PLAN_SEMANTIC_VERSION),
            trace_template_versions: BTreeSet::new(),
            components: BTreeSet::new(),
            phases: BTreeSet::from([ResidualEventPhase::PreDispatchAttempt]),
            evidence: BTreeSet::new(),
            // A scope may terminate on any path and may suspend; every
            // terminal and suspension path must preserve the monitor.
            exits: ResidualScopeExitKind::ALL.into_iter().collect(),
            suspension: ResidualSuspensionKind::ALL.into_iter().collect(),
            obligations: plan.obligations().len(),
            max_alternatives: 0,
            max_clauses: 0,
            max_patterns: 0,
        };
        for obligation in plan.obligations().iter() {
            match plan.component_for(obligation.id()) {
                Some(SemanticPlanComponent::Action(_)) => {
                    requirements
                        .components
                        .insert(ResidualComponentKind::ActionPredicate);
                }
                Some(SemanticPlanComponent::Trace(component)) => match component.monitor() {
                    TraceComponentMonitor::Symbolic(monitor) => {
                        requirements
                            .components
                            .insert(ResidualComponentKind::GenericTraceAlternatives);
                        requirements.trace_template_versions.insert((1, 1));
                        requirements.max_alternatives = requirements
                            .max_alternatives
                            .max(monitor.alternatives().len());
                        let mut patterns = BTreeSet::new();
                        for group in monitor.alternatives() {
                            requirements.max_clauses = requirements.max_clauses.max(group.len());
                            for rule in group {
                                match rule {
                                    SymbolicTraceRule::Allow(pattern)
                                    | SymbolicTraceRule::Deny(pattern) => {
                                        patterns.insert(pattern.fingerprint());
                                    }
                                    SymbolicTraceRule::Before { guard, target } => {
                                        patterns.insert(guard.fingerprint());
                                        patterns.insert(target.fingerprint());
                                        requirements
                                            .phases
                                            .insert(ResidualEventPhase::CorrelatedOutcome);
                                        requirements
                                            .evidence
                                            .insert(ResidualEvidenceKind::AcceptedPriorOutcome);
                                    }
                                }
                            }
                        }
                        requirements.max_patterns = requirements.max_patterns.max(patterns.len());
                    }
                    TraceComponentMonitor::Concrete(monitor) => {
                        requirements
                            .components
                            .insert(ResidualComponentKind::TraceAlternatives);
                        let template = monitor.template();
                        requirements
                            .trace_template_versions
                            .insert((template.schema_version(), template.semantic_version()));
                        let alternatives = template.alternatives();
                        requirements.max_alternatives =
                            requirements.max_alternatives.max(alternatives.len());
                        let mut patterns = BTreeSet::new();
                        for group in alternatives {
                            requirements.max_clauses = requirements.max_clauses.max(group.len());
                            for clause in group {
                                match clause {
                                    TraceAlternativeClause::Allow(pattern)
                                    | TraceAlternativeClause::Deny(pattern) => {
                                        patterns.insert(*pattern);
                                    }
                                    TraceAlternativeClause::Before(prior) => {
                                        patterns.insert(prior.guard());
                                        patterns.insert(prior.target());
                                        requirements
                                            .phases
                                            .insert(ResidualEventPhase::CorrelatedOutcome);
                                        requirements
                                            .evidence
                                            .insert(ResidualEvidenceKind::AcceptedPriorOutcome);
                                    }
                                }
                            }
                        }
                        requirements.max_patterns = requirements.max_patterns.max(patterns.len());
                    }
                },
                // A plan built through `SemanticResidualPlan::build` maps every
                // obligation; a missing component is an unsatisfiable need.
                None => {
                    requirements.obligations = usize::MAX;
                }
            }
        }
        requirements
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResidualSupportError {
    NoResidualSupport,
    PlanVersion((u32, u32)),
    TraceTemplateVersion((u32, u32)),
    Component(ResidualComponentKind),
    Phase(ResidualEventPhase),
    Evidence(ResidualEvidenceKind),
    Exit(ResidualScopeExitKind),
    Suspension(ResidualSuspensionKind),
    ObligationLimit,
    AlternativeLimit,
    ClauseLimit,
    PatternLimit,
}

/// The result of checking one plan against one target's capabilities. It is
/// bound to both identities, carries no authority and is not installation
/// evidence; engine preparation must still bind sites and prove coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResidualBackendSupport {
    plan: PlanHash,
    target: CodeFingerprint,
}

impl ResidualBackendSupport {
    pub fn plan(&self) -> PlanHash {
        self.plan
    }
    pub fn target(&self) -> CodeFingerprint {
        self.target
    }

    /// Rechecks this result against the current plan and target.
    pub fn matches(
        &self,
        plan: &SemanticResidualPlan,
        target: &ResidualTargetCapabilities,
    ) -> bool {
        self.plan == plan.hash() && self.target == target.fingerprint()
    }
}

/// Checks every semantic need of `plan` against `target`, reporting the first
/// unsupported need in a fixed order.
pub fn validate_backend_support(
    plan: &SemanticResidualPlan,
    target: &ResidualTargetCapabilities,
) -> Result<ResidualBackendSupport, ResidualSupportError> {
    use ResidualSupportError as Error;
    if target.is_none() {
        return Err(Error::NoResidualSupport);
    }
    let needs = ResidualPlanRequirements::of(plan);
    if !target.plan_versions.contains(&needs.plan_version) {
        return Err(Error::PlanVersion(needs.plan_version));
    }
    for version in &needs.trace_template_versions {
        if !target.trace_template_versions.contains(version) {
            return Err(Error::TraceTemplateVersion(*version));
        }
    }
    if let Some(kind) = needs.components.difference(&target.components).next() {
        return Err(Error::Component(*kind));
    }
    if let Some(phase) = needs.phases.difference(&target.phases).next() {
        return Err(Error::Phase(*phase));
    }
    if let Some(evidence) = needs.evidence.difference(&target.evidence).next() {
        return Err(Error::Evidence(*evidence));
    }
    if let Some(exit) = needs.exits.difference(&target.exits).next() {
        return Err(Error::Exit(*exit));
    }
    if let Some(kind) = needs.suspension.difference(&target.suspension).next() {
        return Err(Error::Suspension(*kind));
    }
    if needs.obligations > target.limits.max_obligations_per_plan {
        return Err(Error::ObligationLimit);
    }
    if needs.max_alternatives > target.limits.max_alternatives {
        return Err(Error::AlternativeLimit);
    }
    if needs.max_clauses > target.limits.max_clauses {
        return Err(Error::ClauseLimit);
    }
    if needs.max_patterns > target.limits.max_patterns {
        return Err(Error::PatternLimit);
    }
    Ok(ResidualBackendSupport {
        plan: plan.hash(),
        target: target.fingerprint(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> ResidualTargetCapabilities {
        ResidualTargetCapabilities {
            engine: "test-engine".into(),
            engine_semantic_version: 1,
            plan_versions: BTreeSet::from([(PLAN_SCHEMA_VERSION, PLAN_SEMANTIC_VERSION)]),
            trace_template_versions: BTreeSet::from([(1, 1)]),
            components: BTreeSet::from([
                ResidualComponentKind::ActionPredicate,
                ResidualComponentKind::TraceAlternatives,
            ]),
            phases: BTreeSet::from([
                ResidualEventPhase::PreDispatchAttempt,
                ResidualEventPhase::CorrelatedOutcome,
            ]),
            evidence: BTreeSet::from([ResidualEvidenceKind::AcceptedPriorOutcome]),
            exits: ResidualScopeExitKind::ALL.into_iter().collect(),
            suspension: ResidualSuspensionKind::ALL.into_iter().collect(),
            limits: ResidualTargetLimits {
                max_obligations_per_plan: 64,
                max_alternatives: 64,
                max_clauses: 64,
                max_patterns: 64,
            },
        }
    }

    #[test]
    fn capability_fingerprints_cover_every_semantic_field() {
        assert!(ResidualTargetCapabilities::default().is_none());
        let base = full();
        let mut variants = vec![base.clone()];
        let mut push = |change: &dyn Fn(&mut ResidualTargetCapabilities)| {
            let mut value = base.clone();
            change(&mut value);
            variants.push(value);
        };
        push(&|v| v.engine.push('x'));
        push(&|v| v.engine_semantic_version += 1);
        push(&|v| {
            v.plan_versions.insert((9, 9));
        });
        push(&|v| {
            v.trace_template_versions.clear();
        });
        push(&|v| {
            v.components.remove(&ResidualComponentKind::ActionPredicate);
        });
        push(&|v| {
            v.phases.remove(&ResidualEventPhase::CorrelatedOutcome);
        });
        push(&|v| v.evidence.clear());
        push(&|v| {
            v.exits.remove(&ResidualScopeExitKind::Cancellation);
        });
        push(&|v| {
            v.suspension
                .remove(&ResidualSuspensionKind::CheckpointRestore);
        });
        push(&|v| v.limits.max_patterns -= 1);
        let fingerprints = variants
            .iter()
            .map(ResidualTargetCapabilities::fingerprint)
            .collect::<BTreeSet<_>>();
        assert_eq!(fingerprints.len(), variants.len());
    }
}
