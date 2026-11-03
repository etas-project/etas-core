//! Bounded generic trace alternatives. These are not installation evidence.

use std::collections::BTreeMap;

use crate::{
    CodeFingerprint,
    identity::canonical::CanonicalEncoder,
    residual::{
        identity::{GenericArgumentIdentity, GenericParameterId},
        pattern::{ActionPatternTemplate, ActionPatternTemplateError, PatternTemplateLimits},
        type_template::StaticTypeTemplate,
    },
};

use super::{
    AlternativeError, AlternativeLimits, AlternativeRule, CompiledTraceAlternatives,
    MAX_TRACE_TEMPLATE_NODES,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolicTraceRule {
    Allow(ActionPatternTemplate),
    Deny(ActionPatternTemplate),
    Before {
        guard: ActionPatternTemplate,
        target: ActionPatternTemplate,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolicTraceAlternatives {
    fingerprint: CodeFingerprint,
    alternatives: Vec<Vec<SymbolicTraceRule>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolicTraceError {
    EmptyAlternatives,
    EmptyAlternative,
    AlternativeLimit,
    ClauseLimit,
    Pattern {
        alternative: usize,
        clause: usize,
        operand: usize,
        error: ActionPatternTemplateError,
    },
    Compile(AlternativeError),
}

impl SymbolicTraceAlternatives {
    pub fn new(
        alternatives: Vec<Vec<SymbolicTraceRule>>,
        limits: AlternativeLimits,
    ) -> Result<Self, SymbolicTraceError> {
        if alternatives.is_empty() {
            return Err(SymbolicTraceError::EmptyAlternatives);
        }
        if alternatives.len() > limits.max_alternatives
            || alternatives.len() > MAX_TRACE_TEMPLATE_NODES
        {
            return Err(SymbolicTraceError::AlternativeLimit);
        }
        let mut count = alternatives.len();
        let mut clauses = 0usize;
        for group in &alternatives {
            if group.is_empty() {
                return Err(SymbolicTraceError::EmptyAlternative);
            }
            clauses = clauses
                .checked_add(group.len())
                .ok_or(SymbolicTraceError::ClauseLimit)?;
            count = count
                .checked_add(group.len())
                .ok_or(SymbolicTraceError::ClauseLimit)?;
            if clauses > limits.max_clauses || count > MAX_TRACE_TEMPLATE_NODES {
                return Err(SymbolicTraceError::ClauseLimit);
            }
        }
        let mut encoder = CanonicalEncoder::new("etas.code.residual.symbolic-trace.v1");
        encoder.number(alternatives.len() as u64);
        for group in &alternatives {
            encoder.number(group.len() as u64);
            for rule in group {
                match rule {
                    SymbolicTraceRule::Allow(pattern) => {
                        encoder.tag(1);
                        encoder.fingerprint(pattern.fingerprint());
                    }
                    SymbolicTraceRule::Deny(pattern) => {
                        encoder.tag(2);
                        encoder.fingerprint(pattern.fingerprint());
                    }
                    SymbolicTraceRule::Before { guard, target } => {
                        encoder.tag(3);
                        encoder.fingerprint(guard.fingerprint());
                        encoder.fingerprint(target.fingerprint());
                    }
                }
            }
        }
        Ok(Self {
            fingerprint: encoder.finish(),
            alternatives,
        })
    }

    pub fn fingerprint(&self) -> CodeFingerprint {
        self.fingerprint
    }
    pub fn alternatives(&self) -> &[Vec<SymbolicTraceRule>] {
        &self.alternatives
    }

    /// Resolves checked type arguments before compiling the shared concrete
    /// automaton. Caller must validate the binding provenance and sites.
    pub fn instantiate(
        &self,
        bindings: &BTreeMap<GenericParameterId, GenericArgumentIdentity>,
        structures: &BTreeMap<GenericParameterId, StaticTypeTemplate>,
        pattern_limits: PatternTemplateLimits,
        monitor_limits: AlternativeLimits,
    ) -> Result<CompiledTraceAlternatives, SymbolicTraceError> {
        // Revalidate limits on use rather than trusting the construction target.
        if self.alternatives.len() > monitor_limits.max_alternatives {
            return Err(SymbolicTraceError::AlternativeLimit);
        }
        let mut count = 0usize;
        for group in &self.alternatives {
            count = count
                .checked_add(group.len())
                .ok_or(SymbolicTraceError::ClauseLimit)?;
            if count > monitor_limits.max_clauses {
                return Err(SymbolicTraceError::ClauseLimit);
            }
        }
        let mut remaining = pattern_limits.max_binding_steps;
        let mut result = Vec::with_capacity(self.alternatives.len());
        for (alternative, group) in self.alternatives.iter().enumerate() {
            let mut rules = Vec::with_capacity(group.len());
            for (clause, rule) in group.iter().enumerate() {
                let mut make = |pattern: &ActionPatternTemplate, operand| {
                    pattern
                        .instantiate_with_budget(
                            bindings,
                            structures,
                            pattern_limits,
                            &mut remaining,
                        )
                        .map_err(|error| SymbolicTraceError::Pattern {
                            alternative,
                            clause,
                            operand,
                            error,
                        })
                };
                rules.push(match rule {
                    SymbolicTraceRule::Allow(pattern) => AlternativeRule::Allow(make(pattern, 0)?),
                    SymbolicTraceRule::Deny(pattern) => AlternativeRule::Deny(make(pattern, 0)?),
                    SymbolicTraceRule::Before { guard, target } => AlternativeRule::Before {
                        guard: make(guard, 0)?,
                        target: make(target, 1)?,
                    },
                });
            }
            result.push(rules);
        }
        CompiledTraceAlternatives::compile(&result, monitor_limits)
            .map_err(SymbolicTraceError::Compile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DefinitionKey, DefinitionKind, PackageKey,
        descriptor::{DescriptorLimits, EffectTargetInfo},
        residual::{
            monitor::ConcreteEvidenceLimits,
            pattern::{ActionPatternLimits, ActionSelectorTemplate},
            type_template::TypeTemplateLimits,
        },
    };

    fn limits() -> (PatternTemplateLimits, AlternativeLimits) {
        let pattern = ActionPatternLimits {
            max_selectors: 4,
            max_identity_segments: 16,
            max_identity_bytes: 128,
            max_literal_bytes: 128,
        };
        (
            PatternTemplateLimits {
                pattern,
                types: TypeTemplateLimits::default(),
                descriptor: DescriptorLimits::default(),
                max_binding_steps: 100,
            },
            AlternativeLimits {
                evidence: ConcreteEvidenceLimits {
                    pattern,
                    max_bindings: 4,
                    max_value_bytes: 128,
                },
                max_alternatives: 4,
                max_clauses: 8,
                max_patterns: 8,
                max_checks_per_event: 32,
                max_product_work_per_event: 128,
            },
        )
    }

    fn pattern(name: &str, parameter: GenericParameterId) -> ActionPatternTemplate {
        let owner = DefinitionKey::new(
            PackageKey::from_canonical_fingerprint(CodeFingerprint::from_bytes([7; 32])),
            vec!["app".into()],
            vec!["Gate".into()],
            DefinitionKind::Effect,
        )
        .unwrap();
        ActionPatternTemplate::new(
            EffectTargetInfo::Action {
                effect: owner,
                name: name.into(),
            },
            vec![ActionSelectorTemplate::TypeParameter(parameter)],
            vec![],
            limits().0,
        )
        .unwrap()
    }

    #[test]
    fn preserves_complete_alternatives_and_prior_direction_after_binding() {
        let t =
            GenericParameterId::from_canonical_fingerprint(CodeFingerprint::from_bytes([1; 32]));
        let guard = pattern("guard", t);
        let target = pattern("target", t);
        let deny = pattern("deny", t);
        let (pattern_limits, monitor_limits) = limits();
        let symbolic = SymbolicTraceAlternatives::new(
            vec![
                vec![SymbolicTraceRule::Before {
                    guard: guard.clone(),
                    target: target.clone(),
                }],
                vec![SymbolicTraceRule::Deny(deny.clone())],
            ],
            monitor_limits,
        )
        .unwrap();
        let bindings = BTreeMap::from([(
            t,
            GenericArgumentIdentity::Type(CodeFingerprint::from_bytes([3; 32])),
        )]);
        let concrete = symbolic
            .instantiate(&bindings, &BTreeMap::new(), pattern_limits, monitor_limits)
            .unwrap();
        let resolve = |value: &ActionPatternTemplate| {
            value
                .instantiate(&bindings, &BTreeMap::new(), pattern_limits)
                .unwrap()
        };
        let expected = CompiledTraceAlternatives::compile(
            &[
                vec![AlternativeRule::Before {
                    guard: resolve(&guard),
                    target: resolve(&target),
                }],
                vec![AlternativeRule::Deny(resolve(&deny))],
            ],
            monitor_limits,
        )
        .unwrap();
        assert_eq!(concrete.template(), expected.template());
        let reversed = SymbolicTraceAlternatives::new(
            vec![
                vec![SymbolicTraceRule::Before {
                    guard: target,
                    target: guard,
                }],
                vec![SymbolicTraceRule::Deny(deny)],
            ],
            monitor_limits,
        )
        .unwrap();
        assert_ne!(symbolic.fingerprint(), reversed.fingerprint());
        assert_ne!(
            concrete.template_id(),
            reversed
                .instantiate(&bindings, &BTreeMap::new(), pattern_limits, monitor_limits)
                .unwrap()
                .template_id()
        );
        assert!(
            matches!(symbolic.instantiate(&BTreeMap::new(), &BTreeMap::new(), pattern_limits, monitor_limits), Err(SymbolicTraceError::Pattern { alternative: 0, clause: 0, operand: 0, error: ActionPatternTemplateError::MissingBinding(id) }) if id == t)
        );
        assert!(matches!(
            symbolic.instantiate(
                &bindings,
                &BTreeMap::new(),
                PatternTemplateLimits {
                    max_binding_steps: 2,
                    ..pattern_limits
                },
                monitor_limits
            ),
            Err(SymbolicTraceError::Pattern {
                alternative: 0,
                clause: 0,
                operand: 1,
                error: ActionPatternTemplateError::ResourceLimit
            })
        ));
    }
}
