use std::{collections::BTreeMap, sync::Arc};

use crate::{
    CodeFingerprint, DefinitionKey, DefinitionKind, identity::canonical::CanonicalEncoder,
};

use super::{
    DescriptorError, DescriptorGraph, DescriptorNode, EffectRowInfoRef, EffectTargetInfo, SpecInfo,
    TypeInfoRef,
};

/// Attached constraints are descriptions, not satisfaction evidence. An inline
/// constraint has no declaration identity and must not be given a fabricated one.
#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum AttachedSpecInfo {
    Named(SpecInfo),
    InlineTrace(Arc<TraceConstraintInfo>),
    Protocol {
        definition: DefinitionKey,
        arguments: Vec<TypeInfoRef>,
    },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct TraceConstraintInfo {
    pub alternatives: Vec<Vec<TraceRuleInfo>>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub enum TraceRuleInfo {
    Allow(TracePatternInfo),
    Deny(TracePatternInfo),
    RequireBefore {
        guard: TracePatternInfo,
        target: TracePatternInfo,
    },
    RequireAfter {
        target: TracePatternInfo,
        obligation: TracePatternInfo,
    },
    Limit {
        count: usize,
    },
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct TracePatternInfo {
    pub row: EffectRowInfoRef,
    pub bounds: Vec<TraceSelectorBoundInfo>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct TraceSelectorBoundInfo {
    pub target: EffectTargetInfo,
    pub argument: u32,
    pub spec: SpecInfo,
}

impl TraceRuleInfo {
    pub fn patterns(&self) -> impl Iterator<Item = &TracePatternInfo> {
        let patterns = match self {
            Self::Allow(pattern) | Self::Deny(pattern) => [Some(pattern), None],
            Self::RequireBefore { guard, target } => [Some(guard), Some(target)],
            Self::RequireAfter { target, obligation } => [Some(target), Some(obligation)],
            Self::Limit { .. } => [None, None],
        };
        patterns.into_iter().flatten()
    }

    fn patterns_mut(&mut self) -> impl Iterator<Item = &mut TracePatternInfo> {
        let patterns = match self {
            Self::Allow(pattern) | Self::Deny(pattern) => [Some(pattern), None],
            Self::RequireBefore { guard, target } => [Some(guard), Some(target)],
            Self::RequireAfter { target, obligation } => [Some(target), Some(obligation)],
            Self::Limit { .. } => [None, None],
        };
        patterns.into_iter().flatten()
    }
}

impl AttachedSpecInfo {
    pub fn validate(&self, graph: &DescriptorGraph) -> Result<(), DescriptorError> {
        match self {
            Self::Named(spec) => validate_spec(graph, spec),
            Self::InlineTrace(constraint) => constraint.validate(graph),
            Self::Protocol {
                definition,
                arguments,
            } => {
                if definition.kind() != DefinitionKind::Protocol {
                    return Err(DescriptorError::InvalidDescription {
                        reason: "protocol constraint has a non-protocol identity",
                    });
                }
                for ty in arguments {
                    if graph.type_info(*ty).is_none() {
                        return Err(DescriptorError::MissingReference {
                            from: DescriptorNode::Type(*ty),
                            target: DescriptorNode::Type(*ty),
                        });
                    }
                }
                Ok(())
            }
        }
    }
    pub fn fingerprint(&self, graph: &DescriptorGraph) -> Result<CodeFingerprint, DescriptorError> {
        let mut encoder = CanonicalEncoder::new("etas.code.attached-spec.v1");
        match self {
            Self::Named(spec) => {
                encoder.tag(0);
                encode_spec(&mut encoder, graph, spec)?;
            }
            Self::InlineTrace(constraint) => {
                encoder.tag(1);
                encoder.fingerprint(constraint.fingerprint(graph)?);
            }
            Self::Protocol {
                definition,
                arguments,
            } => {
                if definition.kind() != DefinitionKind::Protocol {
                    return Err(DescriptorError::InvalidDescription {
                        reason: "protocol constraint has a non-protocol identity",
                    });
                }
                encoder.tag(2);
                encoder.fingerprint(definition.fingerprint());
                encoder.number(arguments.len() as u64);
                for argument in arguments {
                    encoder.fingerprint(graph.type_fingerprint(*argument)?);
                }
            }
        }
        Ok(encoder.finish())
    }
}

impl TraceConstraintInfo {
    pub fn validate(&self, graph: &DescriptorGraph) -> Result<(), DescriptorError> {
        for pattern in self
            .alternatives
            .iter()
            .flatten()
            .flat_map(TraceRuleInfo::patterns)
        {
            let row = graph
                .effect_row(pattern.row)
                .ok_or(DescriptorError::MissingReference {
                    from: DescriptorNode::Row(pattern.row),
                    target: DescriptorNode::Row(pattern.row),
                })?;
            for bound in &pattern.bounds {
                if !row.effects.iter().any(|effect| {
                    effect.target == bound.target
                        && (bound.argument as usize) < effect.arguments.len()
                }) {
                    return Err(DescriptorError::InvalidDescription {
                        reason: "trace selector bound does not reference an argument of its pattern",
                    });
                }
                validate_spec(graph, &bound.spec)?;
            }
        }
        Ok(())
    }

    /// The runtime can share a catalog graph. Export includes only this
    /// constraint's row/type closures, never unrelated private declarations.
    pub fn project(
        &self,
        graph: &DescriptorGraph,
    ) -> Result<(DescriptorGraph, Self), DescriptorError> {
        self.validate(graph)?;
        let mut roots = Vec::new();
        for pattern in self
            .alternatives
            .iter()
            .flatten()
            .flat_map(TraceRuleInfo::patterns)
        {
            roots.push(DescriptorNode::Row(pattern.row));
            for bound in &pattern.bounds {
                roots.extend(
                    bound
                        .spec
                        .arguments
                        .iter()
                        .copied()
                        .map(DescriptorNode::Type),
                );
            }
        }
        let (graph, projected) = graph.project_roots(&roots)?;
        let remap = roots.into_iter().zip(projected).collect::<BTreeMap<_, _>>();
        let mut value = self.clone();
        for pattern in value
            .alternatives
            .iter_mut()
            .flatten()
            .flat_map(TraceRuleInfo::patterns_mut)
        {
            let Some(DescriptorNode::Row(row)) = remap.get(&DescriptorNode::Row(pattern.row))
            else {
                return Err(DescriptorError::InvalidDescription {
                    reason: "trace row projection is incomplete",
                });
            };
            pattern.row = *row;
            for bound in &mut pattern.bounds {
                for argument in &mut bound.spec.arguments {
                    let Some(DescriptorNode::Type(ty)) =
                        remap.get(&DescriptorNode::Type(*argument))
                    else {
                        return Err(DescriptorError::InvalidDescription {
                            reason: "trace bound projection is incomplete",
                        });
                    };
                    *argument = *ty;
                }
            }
        }
        Ok((graph, value))
    }

    pub fn fingerprint(&self, graph: &DescriptorGraph) -> Result<CodeFingerprint, DescriptorError> {
        self.validate(graph)?;
        let mut encoder = CanonicalEncoder::new("etas.code.trace-constraint.v1");
        encoder.number(self.alternatives.len() as u64);
        for alternative in &self.alternatives {
            encoder.number(alternative.len() as u64);
            for rule in alternative {
                let tag = match rule {
                    TraceRuleInfo::Allow(_) => 0,
                    TraceRuleInfo::Deny(_) => 1,
                    TraceRuleInfo::RequireBefore { .. } => 2,
                    TraceRuleInfo::RequireAfter { .. } => 3,
                    TraceRuleInfo::Limit { .. } => 4,
                };
                encoder.tag(tag);
                if let TraceRuleInfo::Limit { count } = rule {
                    encoder.number(*count as u64);
                }
                for pattern in rule.patterns() {
                    encoder.fingerprint(graph.row_fingerprint(pattern.row)?);
                    encoder.number(pattern.bounds.len() as u64);
                    for bound in &pattern.bounds {
                        match &bound.target {
                            EffectTargetInfo::Effect(identity) => {
                                encoder.tag(0);
                                encoder.fingerprint(identity.fingerprint());
                            }
                            EffectTargetInfo::Action { effect, name } => {
                                encoder.tag(1);
                                encoder.fingerprint(effect.fingerprint());
                                encoder.bytes(name.as_bytes());
                            }
                        }
                        encoder.number(u64::from(bound.argument));
                        encode_spec(&mut encoder, graph, &bound.spec)?;
                    }
                }
            }
        }
        Ok(encoder.finish())
    }
}

fn validate_spec(graph: &DescriptorGraph, spec: &SpecInfo) -> Result<(), DescriptorError> {
    if spec.definition.kind() != DefinitionKind::Spec {
        return Err(DescriptorError::InvalidDescription {
            reason: "spec descriptor has a non-spec identity",
        });
    }
    for ty in &spec.arguments {
        if graph.type_info(*ty).is_none() {
            return Err(DescriptorError::MissingReference {
                from: DescriptorNode::Type(*ty),
                target: DescriptorNode::Type(*ty),
            });
        }
    }
    Ok(())
}

fn encode_spec(
    encoder: &mut CanonicalEncoder,
    graph: &DescriptorGraph,
    spec: &SpecInfo,
) -> Result<(), DescriptorError> {
    validate_spec(graph, spec)?;
    encoder.fingerprint(spec.definition.fingerprint());
    encoder.number(spec.arguments.len() as u64);
    for ty in &spec.arguments {
        encoder.fingerprint(graph.type_fingerprint(*ty)?);
    }
    Ok(())
}
