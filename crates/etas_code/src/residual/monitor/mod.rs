mod action;
mod alternatives;
mod compile;
mod matcher;
mod normalize;
mod symbolic;
mod template;
mod trace_component;

pub use action::{
    ActionComponentError, ActionComponentInput, ActionComponentLimits, CompiledActionPredicate,
};
pub use alternatives::{
    AlternativeBranchData, AlternativeDecision, AlternativeDelta, AlternativeError,
    AlternativeLimits, AlternativeRule, AlternativeScope, AlternativeState, AlternativeStateData,
    CompiledTraceAlternatives, EvidenceRequirements, PendingAlternativeEvidence,
    PriorComponentData, ScopeCheckResult, ScopeExit, ScopeInvocationId,
};
pub use compile::{PriorEvidenceLabel, PriorEvidenceState, compile_prior_evidence};
pub use matcher::{
    AcceptedPriorEvidence, ActionAttemptId, ConcreteActionAttempt, ConcreteEvidenceLimits,
    ConcretePriorEvidence, EvidenceQuery, EvidenceStatus, PriorEvidenceRequirement,
    PriorEvidenceStep, PriorEvidenceStepError, SemanticEventCursor, step_concrete_prior_evidence,
};
pub use normalize::{
    NormalizeError, NormalizedTraceAlternative, PriorRequirement, TraceClause,
    normalize_trace_alternative,
};
pub use symbolic::{SymbolicTraceAlternatives, SymbolicTraceError, SymbolicTraceRule};
pub use template::{
    MAX_TRACE_TEMPLATE_NODES, PriorEvidenceTemplate, PriorEvidenceTemplateError,
    TraceAlternativeClause, TraceAlternativesTemplate,
};
pub use trace_component::{
    BoundTraceComponent, CompiledTraceComponent, TraceBatchCommitError, TraceComponentDecision,
    TraceComponentDelta, TraceComponentError, TraceComponentEvidenceRequirements,
    TraceComponentLimits, TraceComponentMonitor, TraceComponentState, TraceComponentTransition,
    commit_trace_transitions,
};
