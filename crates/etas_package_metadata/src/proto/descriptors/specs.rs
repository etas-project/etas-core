use super::{effects::*, identities::*, invalid, required, schema as p};
use crate::MetadataArtifactError as Error;
use etas_code::descriptor::*;
use std::sync::Arc;

fn encode_pattern(pattern: &TracePatternInfo) -> p::TracePattern {
    p::TracePattern {
        row: pattern.row.0,
        bounds: pattern
            .bounds
            .iter()
            .map(|bound| p::SelectorBound {
                target: Some(encode_target(&bound.target)),
                argument: bound.argument,
                spec: Some(encode_spec(&bound.spec)),
            })
            .collect(),
    }
}
fn decode_pattern(pattern: p::TracePattern) -> Result<TracePatternInfo, Error> {
    Ok(TracePatternInfo {
        row: EffectRowInfoRef(pattern.row),
        bounds: pattern
            .bounds
            .into_iter()
            .map(|bound| {
                Ok(TraceSelectorBoundInfo {
                    target: decode_target(required(bound.target, "selector bound target")?)?,
                    argument: bound.argument,
                    spec: decode_spec(required(bound.spec, "selector spec bound")?)?,
                })
            })
            .collect::<Result<_, Error>>()?,
    })
}

pub(super) fn encode(spec: &AttachedSpecInfo) -> p::AttachedSpec {
    use p::attached_spec::Kind as K;
    p::AttachedSpec {
        kind: Some(match spec {
            AttachedSpecInfo::Named(spec) => K::Named(encode_spec(spec)),
            AttachedSpecInfo::Protocol {
                definition,
                arguments,
            } => K::Protocol(p::Spec {
                definition: Some(encode_key(definition)),
                arguments: arguments.iter().map(|ty| ty.0).collect(),
            }),
            AttachedSpecInfo::InlineTrace(constraint) => K::Inline(p::TraceConstraint {
                alternatives: constraint
                    .alternatives
                    .iter()
                    .map(|rules| p::TraceRules {
                        rules: rules
                            .iter()
                            .map(|rule| p::TraceRule {
                                kind: match rule {
                                    TraceRuleInfo::Allow(_) => 1,
                                    TraceRuleInfo::Deny(_) => 2,
                                    TraceRuleInfo::RequireBefore { .. } => 3,
                                    TraceRuleInfo::RequireAfter { .. } => 4,
                                    TraceRuleInfo::Limit { .. } => 5,
                                },
                                patterns: rule.patterns().map(encode_pattern).collect(),
                                count: match rule {
                                    TraceRuleInfo::Limit { count } => Some(*count as u64),
                                    _ => None,
                                },
                            })
                            .collect(),
                    })
                    .collect(),
            }),
        }),
    }
}

pub(super) fn decode(spec: p::AttachedSpec) -> Result<AttachedSpecInfo, Error> {
    use p::attached_spec::Kind as K;
    Ok(match required(spec.kind, "attached spec kind")? {
        K::Named(spec) => AttachedSpecInfo::Named(decode_spec(spec)?),
        K::Protocol(spec) => AttachedSpecInfo::Protocol {
            definition: decode_key(required(spec.definition, "protocol identity")?)?,
            arguments: spec.arguments.into_iter().map(TypeInfoRef).collect(),
        },
        K::Inline(constraint) => AttachedSpecInfo::InlineTrace(Arc::new(TraceConstraintInfo {
            alternatives: constraint
                .alternatives
                .into_iter()
                .map(|alternative| {
                    alternative
                        .rules
                        .into_iter()
                        .map(|rule| {
                            let patterns = rule
                                .patterns
                                .into_iter()
                                .map(decode_pattern)
                                .collect::<Result<Vec<_>, _>>()?;
                            match (rule.kind, rule.count, patterns.as_slice()) {
                                (1, None, [pattern]) => Ok(TraceRuleInfo::Allow(pattern.clone())),
                                (2, None, [pattern]) => Ok(TraceRuleInfo::Deny(pattern.clone())),
                                (3, None, [guard, target]) => Ok(TraceRuleInfo::RequireBefore {
                                    guard: guard.clone(),
                                    target: target.clone(),
                                }),
                                (4, None, [target, obligation]) => {
                                    Ok(TraceRuleInfo::RequireAfter {
                                        target: target.clone(),
                                        obligation: obligation.clone(),
                                    })
                                }
                                (5, Some(count), []) => Ok(TraceRuleInfo::Limit {
                                    count: usize::try_from(count).map_err(|_| {
                                        invalid("trace limit does not fit the target ABI")
                                    })?,
                                }),
                                _ => Err(invalid("invalid trace rule kind, arity or limit field")),
                            }
                        })
                        .collect::<Result<Vec<_>, Error>>()
                })
                .collect::<Result<Vec<_>, _>>()?,
        })),
    })
}
