//! Inert descriptions of checked declarations, never type or authority evidence.
//! References address this graph only; no compiler arena identities cross this API.

mod annotation;
pub mod behavior;
pub mod callable;
mod definitions;
mod fingerprint;
mod graph;
mod projection;
mod spec;
mod traversal;
pub mod types;
mod validation;

pub use annotation::{
    AnnotationArgumentInfo, AnnotationConstructorKind, AnnotationFieldInfo, AnnotationInfo,
    AnnotationLiteralInfo, AnnotationReferenceInfo, AnnotationSequenceKind, StaticValueInfo,
};
pub use behavior::{
    CallableAnalysisInfo, CallableOutcomeInfo, DeterminismInfo, EffectInfo, EffectRowInfo,
    EffectRowInfoRef, EffectSummaryInfo, EffectTargetInfo, ParameterArgumentInfo,
    ParameterCallInfo, ParameterHandlerArmInfo, ParameterHandlerCompletionInfo,
    ParameterHandlerScopeInfo, ReturnObservationInfo, StaticEffectArgInfo, TraceInfo, TraceInfoRef,
};
pub use callable::{
    GenericOwner, GenericParameterInfo, GenericParameterKind, GenericParameterRef, ParameterInfo,
    SignatureInfo, SpecInfo,
};
pub use definitions::{
    AgentHeaderInfo, AgentMethodInfo, BodyAccess, CallableBodyInfo, CallableHeaderInfo,
    DefinitionBodies, DefinitionDescriptorSet, DefinitionHeaders, DefinitionSnapshotInfo,
    DefinitionSnapshots, DefinitionTypeBinding,
};
pub use graph::{DescriptorGraph, DescriptorGraphInput, DescriptorLimits};
pub use spec::{
    AttachedSpecInfo, TraceConstraintInfo, TracePatternInfo, TraceRuleInfo, TraceSelectorBoundInfo,
};
pub use types::{
    BuiltinTypeInfo, FieldInfo, PrimitiveInfo, TypeInfo, TypeInfoRef, VariantInfo,
    VariantPayloadInfo,
};
pub use validation::{DescriptorError, DescriptorNode};
