use etas_code::descriptor::*;

use super::{effects::*, identities::*, invalid, required, schema as p};
use crate::MetadataArtifactError as Error;

fn encode_call(call: &ParameterCallInfo) -> p::ParameterCall {
    p::ParameterCall {
        callable: Some(encode_callable(&call.callable)),
        signature: call.signature.0,
        position: call.position,
    }
}
fn decode_call(call: p::ParameterCall) -> Result<ParameterCallInfo, Error> {
    Ok(ParameterCallInfo {
        callable: decode_callable(required(call.callable, "parameter call identity")?)?,
        signature: TypeInfoRef(call.signature),
        position: call.position,
    })
}

fn encode_outcome(outcome: &CallableOutcomeInfo) -> p::Outcome {
    use p::outcome::Kind as K;
    p::Outcome {
        kind: Some(match outcome {
            CallableOutcomeInfo::Returned { ty, observation } => K::Returned(p::Returned {
                ty: ty.0,
                observation: Some(p::Observation {
                    kind: Some(match observation {
                        ReturnObservationInfo::Any => p::observation::Kind::Any(true),
                        ReturnObservationInfo::Unit => p::observation::Kind::Unit(true),
                        ReturnObservationInfo::Bool(value) => p::observation::Kind::Bool(*value),
                        ReturnObservationInfo::Variant { owner, name } => {
                            p::observation::Kind::Variant(p::VariantObservation {
                                owner: Some(encode_key(owner)),
                                name: name.clone(),
                            })
                        }
                    }),
                }),
            }),
            CallableOutcomeInfo::Raised { error } => K::Raised(error.0),
            CallableOutcomeInfo::NonReturn => K::NonReturn(true),
        }),
    }
}

fn decode_outcome(outcome: p::Outcome) -> Result<CallableOutcomeInfo, Error> {
    use p::outcome::Kind as K;
    Ok(match required(outcome.kind, "callable outcome")? {
        K::Returned(value) => CallableOutcomeInfo::Returned {
            ty: TypeInfoRef(value.ty),
            observation: match required(
                required(value.observation, "return observation")?.kind,
                "return observation kind",
            )? {
                p::observation::Kind::Any(true) => ReturnObservationInfo::Any,
                p::observation::Kind::Unit(true) => ReturnObservationInfo::Unit,
                p::observation::Kind::Bool(value) => ReturnObservationInfo::Bool(value),
                p::observation::Kind::Variant(value) => ReturnObservationInfo::Variant {
                    owner: decode_key(required(value.owner, "return variant owner")?)?,
                    name: value.name,
                },
                p::observation::Kind::Any(false) | p::observation::Kind::Unit(false) => {
                    return Err(invalid("invalid return observation marker"));
                }
            },
        },
        K::Raised(error) => CallableOutcomeInfo::Raised {
            error: TypeInfoRef(error),
        },
        K::NonReturn(true) => CallableOutcomeInfo::NonReturn,
        K::NonReturn(false) => return Err(invalid("invalid non-return marker")),
    })
}

pub(super) fn encode(trace: &TraceInfo) -> p::Trace {
    use p::trace::Kind as K;
    p::Trace {
        kind: Some(match trace {
            TraceInfo::Empty => K::Empty(true),
            TraceInfo::Event(effect) => K::Event(encode_effect(effect)),
            TraceInfo::Outcome(outcome) => K::Outcome(encode_outcome(outcome)),
            TraceInfo::ParameterCall(call) => K::ParameterCall(encode_call(call)),
            TraceInfo::Seq(values) => K::Seq(p::Refs {
                values: values.iter().map(|r| r.0).collect(),
            }),
            TraceInfo::Choice(values) => K::Choice(p::Refs {
                values: values.iter().map(|r| r.0).collect(),
            }),
            TraceInfo::Repeat(value) => K::Repeat(value.0),
            TraceInfo::UnknownOrder(value) => K::UnknownOrder(value.0),
            TraceInfo::Widened {
                actions,
                parameter_calls,
            } => K::Widened(p::Widened {
                actions: actions.0,
                parameter_calls: parameter_calls.iter().map(encode_call).collect(),
            }),
        }),
    }
}

pub(super) fn decode(trace: p::Trace) -> Result<TraceInfo, Error> {
    use p::trace::Kind as K;
    Ok(match required(trace.kind, "trace kind")? {
        K::Empty(true) => TraceInfo::Empty,
        K::Empty(false) => return Err(invalid("invalid empty trace marker")),
        K::Event(effect) => TraceInfo::Event(decode_effect(effect)?),
        K::Outcome(outcome) => TraceInfo::Outcome(decode_outcome(outcome)?),
        K::ParameterCall(call) => TraceInfo::ParameterCall(decode_call(call)?),
        K::Seq(values) => TraceInfo::Seq(values.values.into_iter().map(TraceInfoRef).collect()),
        K::Choice(values) => {
            TraceInfo::Choice(values.values.into_iter().map(TraceInfoRef).collect())
        }
        K::Repeat(value) => TraceInfo::Repeat(TraceInfoRef(value)),
        K::UnknownOrder(value) => TraceInfo::UnknownOrder(EffectRowInfoRef(value)),
        K::Widened(value) => TraceInfo::Widened {
            actions: EffectRowInfoRef(value.actions),
            parameter_calls: value
                .parameter_calls
                .into_iter()
                .map(decode_call)
                .collect::<Result<_, _>>()?,
        },
    })
}
