use crate::descriptor::{
    CallableOutcomeInfo, DescriptorError, DescriptorGraph, EffectInfo, EffectTargetInfo,
    ReturnObservationInfo, StaticEffectArgInfo,
};
use crate::{CallableId, CodeFingerprint, DefinitionKey};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub enum CanonicalSelector {
    Type(CodeFingerprint),
    Resource(DefinitionKey),
    Callable(CallableId),
    String(String),
    Integer(String),
    Wildcard,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct CanonicalEffect {
    pub target: EffectTargetInfo,
    pub arguments: Vec<CanonicalSelector>,
}

impl CanonicalEffect {
    /// Uniform coverage, not a test for nonempty intersection. Partial overlap
    /// must never contribute an NFA destination for a symbolic candidate event.
    pub fn covers(&self, candidate: &Self) -> bool {
        let target = self.target == candidate.target
            || matches!(
                (&self.target, &candidate.target),
                (EffectTargetInfo::Effect(bound), EffectTargetInfo::Action { effect, .. }) if bound == effect
            );
        target
            && self.arguments.len() == candidate.arguments.len()
            && self
                .arguments
                .iter()
                .zip(&candidate.arguments)
                .all(|(bound, actual)| {
                    matches!(bound, CanonicalSelector::Wildcard) || bound == actual
                })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub enum TraceLabel {
    Action(CanonicalEffect),
    ParameterCall {
        callable: CallableId,
        position: u32,
        signature: CodeFingerprint,
    },
    Returned {
        ty: CodeFingerprint,
        observation: ReturnObservationInfo,
    },
    Raised {
        error: CodeFingerprint,
    },
    NonReturn,
}

impl TraceLabel {
    pub(super) fn covers(&self, candidate: &Self) -> bool {
        match (self, candidate) {
            (Self::NonReturn, Self::NonReturn) => true,
            (Self::Action(bound), Self::Action(actual)) => bound.covers(actual),
            (
                Self::Returned {
                    ty: expected,
                    observation: bound,
                },
                Self::Returned {
                    ty: actual,
                    observation,
                },
            ) => {
                expected == actual
                    && (matches!(bound, ReturnObservationInfo::Any) || bound == observation)
            }
            _ => self == candidate,
        }
    }
}

pub(super) fn outcome_label(
    graph: &DescriptorGraph,
    outcome: &CallableOutcomeInfo,
) -> Result<TraceLabel, DescriptorError> {
    Ok(match outcome {
        CallableOutcomeInfo::Returned { ty, observation } => TraceLabel::Returned {
            ty: graph.type_fingerprint(*ty)?,
            observation: observation.clone(),
        },
        CallableOutcomeInfo::Raised { error } => TraceLabel::Raised {
            error: graph.type_fingerprint(*error)?,
        },
        CallableOutcomeInfo::NonReturn => TraceLabel::NonReturn,
    })
}

pub fn canonical_effect(
    graph: &DescriptorGraph,
    effect: &EffectInfo,
) -> Result<CanonicalEffect, DescriptorError> {
    let arguments = effect
        .arguments
        .iter()
        .map(|argument| {
            Ok(match argument {
                StaticEffectArgInfo::Type(ty) => {
                    CanonicalSelector::Type(graph.type_fingerprint(*ty)?)
                }
                StaticEffectArgInfo::Resource(key) => CanonicalSelector::Resource(key.clone()),
                StaticEffectArgInfo::Callable(id) => CanonicalSelector::Callable(id.clone()),
                StaticEffectArgInfo::String(value) => CanonicalSelector::String(value.clone()),
                StaticEffectArgInfo::Integer(value) => CanonicalSelector::Integer(value.clone()),
                StaticEffectArgInfo::Wildcard => CanonicalSelector::Wildcard,
            })
        })
        .collect::<Result<_, DescriptorError>>()?;
    Ok(CanonicalEffect {
        target: effect.target.clone(),
        arguments,
    })
}
