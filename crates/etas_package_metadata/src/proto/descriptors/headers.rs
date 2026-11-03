use super::{annotations, identities::*, invalid, required, schema as p, snapshots, specs};
use crate::MetadataArtifactError as Error;
use etas_code::descriptor::*;

pub(super) fn encode(headers: &DefinitionHeaders) -> p::Headers {
    p::Headers {
        bodies: headers.bodies.as_ref().map(super::bodies::encode),
        snapshots: headers.snapshots.as_ref().map(snapshots::encode),
        callables: headers
            .callables
            .iter()
            .map(|callable| {
                let effects = &callable.analysis.effects;
                p::CallableHeader {
                    id: Some(encode_callable(&callable.id)),
                    signature: callable.signature.0,
                    analysis: Some(p::Analysis {
                        declared_bound: effects.declared_bound.map(|r| r.0),
                        escaping: effects.escaping.map(|r| r.0),
                        requested: effects.requested.map(|r| r.0),
                        default_handled: effects.default_handled.map(|r| r.0),
                        determinism: callable.analysis.determinism.map(|value| match value {
                            DeterminismInfo::Deterministic => 1,
                            DeterminismInfo::RuntimeMediated => 2,
                            DeterminismInfo::NonDeterministic => 3,
                        }),
                        independent_determinism: callable.analysis.independent_determinism.map(
                            |value| match value {
                                DeterminismInfo::Deterministic => 1,
                                DeterminismInfo::RuntimeMediated => 2,
                                DeterminismInfo::NonDeterministic => 3,
                            },
                        ),
                        trace: callable.analysis.trace.map(|r| r.0),
                        outcomes: callable.analysis.outcomes.map(|r| r.0),
                    }),
                }
            })
            .collect(),
        agents: headers
            .agents
            .iter()
            .map(|agent| p::AgentHeader {
                identity: Some(encode_key(&agent.identity)),
                component: agent.component.0,
                default_entry: agent.default_entry.as_ref().map(encode_callable),
                methods: agent.methods.iter().map(encode_callable).collect(),
                tools: agent.tools.iter().map(encode_key).collect(),
                annotations: agent.annotations.iter().map(annotations::encode).collect(),
                specs: agent.specs.iter().map(specs::encode).collect(),
                method_info: agent.method_info.as_ref().map(|info| p::AgentMethods {
                    methods: info
                        .iter()
                        .map(|method| p::AgentMethod {
                            callable: Some(encode_callable(&method.callable)),
                            inherent: method.inherent,
                            is_public: Some(method.is_public),
                            specs: method
                                .specs
                                .iter()
                                .map(super::effects::encode_spec)
                                .collect(),
                        })
                        .collect(),
                }),
            })
            .collect(),
    }
}

pub(super) fn decode(headers: p::Headers) -> Result<DefinitionHeaders, Error> {
    Ok(DefinitionHeaders {
        bodies: headers.bodies.map(super::bodies::decode).transpose()?,
        snapshots: headers.snapshots.map(snapshots::decode).transpose()?,
        callables: headers
            .callables
            .into_iter()
            .map(|callable| {
                let analysis =
                    required(callable.analysis, "callable analysis availability record")?;
                Ok(CallableHeaderInfo {
                    id: decode_callable(required(callable.id, "callable header identity")?)?,
                    signature: TypeInfoRef(callable.signature),
                    analysis: CallableAnalysisInfo {
                        effects: EffectSummaryInfo {
                            declared_bound: analysis.declared_bound.map(EffectRowInfoRef),
                            escaping: analysis.escaping.map(EffectRowInfoRef),
                            requested: analysis.requested.map(EffectRowInfoRef),
                            default_handled: analysis.default_handled.map(EffectRowInfoRef),
                        },
                        determinism: analysis
                            .determinism
                            .map(|value| match value {
                                1 => Ok(DeterminismInfo::Deterministic),
                                2 => Ok(DeterminismInfo::RuntimeMediated),
                                3 => Ok(DeterminismInfo::NonDeterministic),
                                _ => Err(invalid("unknown determinism descriptor")),
                            })
                            .transpose()?,
                        independent_determinism: analysis
                            .independent_determinism
                            .map(|value| match value {
                                1 => Ok(DeterminismInfo::Deterministic),
                                2 => Ok(DeterminismInfo::RuntimeMediated),
                                3 => Ok(DeterminismInfo::NonDeterministic),
                                _ => Err(invalid("unknown independent determinism descriptor")),
                            })
                            .transpose()?,
                        trace: analysis.trace.map(TraceInfoRef),
                        outcomes: analysis.outcomes.map(TraceInfoRef),
                    },
                })
            })
            .collect::<Result<_, Error>>()?,
        agents: headers
            .agents
            .into_iter()
            .map(|agent| {
                Ok(AgentHeaderInfo {
                    method_info: agent
                        .method_info
                        .map(|info| {
                            info.methods
                                .into_iter()
                                .map(|method| {
                                    Ok(AgentMethodInfo {
                                        callable: decode_callable(required(
                                            method.callable,
                                            "method selection callable",
                                        )?)?,
                                        inherent: method.inherent,
                                        is_public: required(method.is_public, "method visibility")?,
                                        specs: method
                                            .specs
                                            .into_iter()
                                            .map(super::effects::decode_spec)
                                            .collect::<Result<_, Error>>()?,
                                    })
                                })
                                .collect::<Result<Vec<_>, Error>>()
                        })
                        .transpose()?,
                    identity: decode_key(required(agent.identity, "agent header identity")?)?,
                    component: TypeInfoRef(agent.component),
                    default_entry: agent.default_entry.map(decode_callable).transpose()?,
                    methods: agent
                        .methods
                        .into_iter()
                        .map(decode_callable)
                        .collect::<Result<_, _>>()?,
                    tools: agent
                        .tools
                        .into_iter()
                        .map(decode_key)
                        .collect::<Result<_, _>>()?,
                    annotations: agent
                        .annotations
                        .into_iter()
                        .map(annotations::decode)
                        .collect::<Result<_, _>>()?,
                    specs: agent
                        .specs
                        .into_iter()
                        .map(specs::decode)
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}
