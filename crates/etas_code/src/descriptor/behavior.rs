use crate::{CallableId, DefinitionKey};

use super::{GenericParameterRef, TypeInfoRef};

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct EffectRowInfoRef(pub u32);

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum EffectTargetInfo {
    Effect(DefinitionKey),
    Action { effect: DefinitionKey, name: String },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum StaticEffectArgInfo {
    Type(TypeInfoRef),
    Resource(DefinitionKey),
    Callable(CallableId),
    String(String),
    Integer(String),
    Wildcard,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct EffectInfo {
    pub target: EffectTargetInfo,
    pub arguments: Vec<StaticEffectArgInfo>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct EffectRowInfo {
    pub effects: Vec<EffectInfo>,
    pub tail: Option<GenericParameterRef>,
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct EffectSummaryInfo {
    pub declared_bound: Option<EffectRowInfoRef>,
    pub escaping: Option<EffectRowInfoRef>,
    pub requested: Option<EffectRowInfoRef>,
    pub default_handled: Option<EffectRowInfoRef>,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum DeterminismInfo {
    Deterministic,
    RuntimeMediated,
    NonDeterministic,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct TraceInfoRef(pub u32);

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct ParameterCallInfo {
    pub callable: CallableId,
    pub signature: TypeInfoRef,
    pub position: u32,
    /// None means argument values were not proved, not a zero-argument call.
    pub arguments: Option<Vec<ParameterArgumentInfo>>,
    /// Checked surrounding handlers, ordered from innermost to outermost.
    /// None means the handler transformer is unavailable, not handler-free.
    pub handlers: Option<Vec<ParameterHandlerScopeInfo>>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum ParameterHandlerScopeInfo {
    /// Ordered, unconditional action arms with precisely known resume values.
    Resume { arms: Vec<ParameterHandlerArmInfo> },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct ParameterHandlerArmInfo {
    pub action: EffectInfo,
    pub result_type: TypeInfoRef,
    pub value: ReturnObservationInfo,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum ParameterArgumentInfo {
    Bool(bool),
    Integer {
        negative: bool,
        magnitude: u128,
    },
    /// The unchanged value of a checked parameter of the owning callable.
    Input {
        position: u32,
    },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum ReturnObservationInfo {
    Any,
    Unit,
    Bool(bool),
    Variant { owner: DefinitionKey, name: String },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum CallableOutcomeInfo {
    Returned {
        ty: TypeInfoRef,
        observation: ReturnObservationInfo,
    },
    Raised {
        error: TypeInfoRef,
    },
    NonReturn,
    /// A checked callback invocation that does not return to its caller.
    ParameterNonReturn(ParameterCallInfo),
}

/// The existing action-trace abstract domain, not an execution log or a proof.
/// Flat references make validation and traversal bounded without recursive data.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum TraceInfo {
    Empty,
    Event(EffectInfo),
    Outcome(CallableOutcomeInfo),
    ParameterCall(ParameterCallInfo),
    Seq(Vec<TraceInfoRef>),
    Choice(Vec<TraceInfoRef>),
    Repeat(TraceInfoRef),
    UnknownOrder(EffectRowInfoRef),
    Widened {
        actions: EffectRowInfoRef,
        parameter_calls: Vec<ParameterCallInfo>,
    },
}

#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct CallableAnalysisInfo {
    pub effects: EffectSummaryInfo,
    pub determinism: Option<DeterminismInfo>,
    /// The checked contribution excluding unresolved callback parameters.
    #[serde(default)]
    pub independent_determinism: Option<DeterminismInfo>,
    pub trace: Option<TraceInfoRef>,
    pub outcomes: Option<TraceInfoRef>,
}
