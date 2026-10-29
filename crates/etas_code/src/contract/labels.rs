use crate::descriptor::{
    CallableOutcomeInfo, DescriptorError, DescriptorGraph, EffectInfo, EffectTargetInfo,
    ParameterArgumentInfo, ParameterCallInfo, ParameterHandlerScopeInfo, ReturnObservationInfo,
    StaticEffectArgInfo,
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

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub enum HandlerScopeLabel {
    Resume(Vec<ResumeArmLabel>),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct ResumeArmLabel {
    pub action: CanonicalEffect,
    pub result_type: CodeFingerprint,
    pub value: ReturnObservationInfo,
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
        arguments: Option<Vec<ParameterArgumentInfo>>,
        handlers: Option<Vec<HandlerScopeLabel>>,
    },
    Returned {
        ty: CodeFingerprint,
        observation: ReturnObservationInfo,
    },
    Raised {
        error: CodeFingerprint,
    },
    NonReturn,
    ParameterNonReturn {
        callable: CallableId,
        position: u32,
        signature: CodeFingerprint,
        arguments: Option<Vec<ParameterArgumentInfo>>,
        handlers: Option<Vec<HandlerScopeLabel>>,
    },
}

fn path_work(path: &[String]) -> usize {
    path.iter()
        .fold(path.len(), |n, segment| n.saturating_add(segment.len()))
}

pub(super) fn key_work(key: &DefinitionKey) -> usize {
    1usize
        .saturating_add(path_work(key.module()))
        .saturating_add(path_work(key.declaration()))
}

pub(super) fn callable_work(id: &CallableId) -> usize {
    match id.identity_ref() {
        crate::identity::CallableIdentityRef::Declaration(key) => key_work(key),
        crate::identity::CallableIdentityRef::Method(method) => {
            let implementation = method.implementation();
            1usize
                .saturating_add(method.name().len())
                .saturating_add(path_work(implementation.module()))
                .saturating_add(key_work(implementation.subject()))
                .saturating_add(implementation.selected_spec().map_or(0, key_work))
        }
    }
}

impl TraceLabel {
    pub(super) fn comparison_work(&self) -> usize {
        fn effect_work(effect: &CanonicalEffect) -> usize {
            let target = match &effect.target {
                EffectTargetInfo::Effect(key) => key_work(key),
                EffectTargetInfo::Action { effect, name } => {
                    key_work(effect).saturating_add(name.len())
                }
            };
            effect.arguments.iter().fold(target, |work, arg| {
                work.saturating_add(match arg {
                    CanonicalSelector::Resource(key) => key_work(key),
                    CanonicalSelector::Callable(id) => callable_work(id),
                    CanonicalSelector::String(value) | CanonicalSelector::Integer(value) => {
                        value.len().saturating_add(1)
                    }
                    CanonicalSelector::Type(_) | CanonicalSelector::Wildcard => 1,
                })
            })
        }
        fn handlers_work(handlers: &Option<Vec<HandlerScopeLabel>>) -> usize {
            handlers.as_ref().map_or(1, |handlers| {
                handlers.iter().fold(1usize, |work, handler| {
                    let HandlerScopeLabel::Resume(arms) = handler;
                    arms.iter().fold(work.saturating_add(1), |work, arm| {
                        work.saturating_add(effect_work(&arm.action))
                            .saturating_add(1)
                    })
                })
            })
        }
        match self {
            Self::Action(effect) => effect_work(effect),
            Self::ParameterCall {
                callable,
                arguments,
                handlers,
                ..
            }
            | Self::ParameterNonReturn {
                callable,
                arguments,
                handlers,
                ..
            } => callable_work(callable)
                .saturating_add(arguments.as_ref().map_or(1, Vec::len))
                .saturating_add(handlers_work(handlers)),
            Self::Returned {
                observation: ReturnObservationInfo::Variant { owner, name },
                ..
            } => key_work(owner).saturating_add(name.len()),
            Self::Returned { .. } | Self::Raised { .. } | Self::NonReturn => 1,
        }
    }

    pub(super) fn covers(&self, candidate: &Self) -> bool {
        match (self, candidate) {
            (Self::NonReturn, Self::NonReturn) => true,
            (Self::Action(bound), Self::Action(actual)) => bound.covers(actual),
            (Self::ParameterCall { handlers: None, .. }, Self::ParameterCall { .. })
            | (Self::ParameterCall { .. }, Self::ParameterCall { handlers: None, .. })
            | (Self::ParameterNonReturn { handlers: None, .. }, Self::ParameterNonReturn { .. })
            | (Self::ParameterNonReturn { .. }, Self::ParameterNonReturn { handlers: None, .. }) => {
                false
            }
            (
                Self::ParameterCall {
                    arguments: None, ..
                },
                Self::ParameterCall { .. },
            )
            | (
                Self::ParameterCall { .. },
                Self::ParameterCall {
                    arguments: None, ..
                },
            )
            | (
                Self::ParameterNonReturn {
                    arguments: None, ..
                },
                Self::ParameterNonReturn { .. },
            )
            | (
                Self::ParameterNonReturn { .. },
                Self::ParameterNonReturn {
                    arguments: None, ..
                },
            ) => false,
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
        CallableOutcomeInfo::ParameterNonReturn(call) => TraceLabel::ParameterNonReturn {
            callable: call.callable.clone(),
            position: call.position,
            signature: graph.type_fingerprint(call.signature)?,
            arguments: call.arguments.clone(),
            handlers: handler_scope_labels(graph, call)?,
        },
    })
}

pub(super) fn handler_scope_labels(
    graph: &DescriptorGraph,
    call: &ParameterCallInfo,
) -> Result<Option<Vec<HandlerScopeLabel>>, DescriptorError> {
    call.handlers
        .as_ref()
        .map(|handlers| {
            handlers
                .iter()
                .map(|handler| match handler {
                    ParameterHandlerScopeInfo::Resume { arms } => Ok(HandlerScopeLabel::Resume(
                        arms.iter()
                            .map(|arm| {
                                Ok(ResumeArmLabel {
                                    action: canonical_effect(graph, &arm.action)?,
                                    result_type: graph.type_fingerprint(arm.result_type)?,
                                    value: arm.value.clone(),
                                })
                            })
                            .collect::<Result<Vec<_>, DescriptorError>>()?,
                    )),
                })
                .collect()
        })
        .transpose()
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
