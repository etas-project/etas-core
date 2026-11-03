//! Portable data for one prior-evidence component, not an executable plan.

use std::collections::BTreeMap;

use etas_utils::automaton::SymbolicAutomaton;

use super::{PriorEvidenceLabel, PriorEvidenceState, compile_prior_evidence};
use crate::residual::{
    identity::ActionPatternId,
    pattern::{ActionPattern, ActionPatternError, ActionPatternLimits},
};

const SCHEMA_VERSION: u32 = 1;
const SEMANTIC_VERSION: u32 = 1;
/// Absolute decode ceiling. The selected target's limits can be stricter.
pub const MAX_TRACE_TEMPLATE_NODES: usize = 100_000;

/// A complete OR of conjunctive clause groups, containing stable references only.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceAlternativesTemplate {
    schema_version: u32,
    semantic_version: u32,
    #[serde(deserialize_with = "deserialize_alternatives")]
    alternatives: Vec<Vec<TraceAlternativeClause>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TraceAlternativeClause {
    Allow(ActionPatternId),
    Deny(ActionPatternId),
    Before(PriorEvidenceTemplate),
}

impl TraceAlternativesTemplate {
    pub(crate) fn new(alternatives: Vec<Vec<TraceAlternativeClause>>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            semantic_version: SEMANTIC_VERSION,
            alternatives,
        }
    }

    pub(crate) fn alternatives(&self) -> &[Vec<TraceAlternativeClause>] {
        &self.alternatives
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn semantic_version(&self) -> u32 {
        self.semantic_version
    }

    pub(crate) fn validate_version(&self) -> Result<(), PriorEvidenceTemplateError> {
        validate_version(self.schema_version, self.semantic_version)
    }
}

fn deserialize_alternatives<'de, D>(
    deserializer: D,
) -> Result<Vec<Vec<TraceAlternativeClause>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{DeserializeSeed, Error, SeqAccess, Visitor};
    struct Groups;
    struct Group<'a>(&'a mut usize);
    impl<'de> DeserializeSeed<'de> for Group<'_> {
        type Value = Vec<TraceAlternativeClause>;
        fn deserialize<D: serde::Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            struct Clauses<'a>(&'a mut usize);
            impl<'de> Visitor<'de> for Clauses<'_> {
                type Value = Vec<TraceAlternativeClause>;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("bounded trace clauses")
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                    if seq.size_hint().is_some_and(|n| n > *self.0) {
                        return Err(A::Error::custom("trace template node limit exceeded"));
                    }
                    let mut clauses = Vec::new();
                    while let Some(clause) = seq.next_element()? {
                        *self.0 = self.0.checked_sub(1).ok_or_else(|| {
                            A::Error::custom("trace template node limit exceeded")
                        })?;
                        clauses.push(clause);
                    }
                    Ok(clauses)
                }
            }
            deserializer.deserialize_seq(Clauses(self.0))
        }
    }
    impl<'de> Visitor<'de> for Groups {
        type Value = Vec<Vec<TraceAlternativeClause>>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("bounded trace alternatives")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            if seq
                .size_hint()
                .is_some_and(|n| n > MAX_TRACE_TEMPLATE_NODES)
            {
                return Err(A::Error::custom("trace template node limit exceeded"));
            }
            let mut remaining = MAX_TRACE_TEMPLATE_NODES;
            let mut alternatives = Vec::new();
            while let Some(group) = seq.next_element_seed(Group(&mut remaining))? {
                remaining = remaining
                    .checked_sub(1)
                    .ok_or_else(|| A::Error::custom("trace template node limit exceeded"))?;
                alternatives.push(group);
            }
            Ok(alternatives)
        }
    }
    deserializer.deserialize_seq(Groups)
}

/// Stable references only. Decoding this record never establishes coverage,
/// backend support or permission to execute an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriorEvidenceTemplate {
    schema_version: u32,
    semantic_version: u32,
    guard: ActionPatternId,
    target: ActionPatternId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorEvidenceTemplateError {
    UnsupportedSchema(u32),
    UnsupportedSemantics(u32),
    MissingPattern(ActionPatternId),
    PatternIdentityMismatch(ActionPatternId),
    InvalidPattern(ActionPatternError),
}

impl PriorEvidenceTemplate {
    pub fn new(guard: ActionPatternId, target: ActionPatternId) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            semantic_version: SEMANTIC_VERSION,
            guard,
            target,
        }
    }

    pub fn guard(&self) -> ActionPatternId {
        self.guard
    }

    pub fn target(&self) -> ActionPatternId {
        self.target
    }

    pub(crate) fn validate_version(&self) -> Result<(), PriorEvidenceTemplateError> {
        validate_version(self.schema_version, self.semantic_version)
    }

    /// Rebuilds a native automaton from validated patterns and caller limits.
    pub fn rebuild(
        &self,
        patterns: &BTreeMap<ActionPatternId, ActionPattern>,
        limits: ActionPatternLimits,
    ) -> Result<
        SymbolicAutomaton<PriorEvidenceState, PriorEvidenceLabel<ActionPatternId>>,
        PriorEvidenceTemplateError,
    > {
        self.validate_version()?;
        let resolve = |id| {
            let pattern = patterns
                .get(&id)
                .ok_or(PriorEvidenceTemplateError::MissingPattern(id))?;
            if pattern.id() != id {
                return Err(PriorEvidenceTemplateError::PatternIdentityMismatch(id));
            }
            pattern
                .validate_with_limits(limits)
                .map_err(PriorEvidenceTemplateError::InvalidPattern)?;
            Ok(pattern)
        };
        resolve(self.guard)?;
        resolve(self.target)?;
        Ok(compile_prior_evidence(&self.guard, &self.target))
    }
}

fn validate_version(schema: u32, semantic: u32) -> Result<(), PriorEvidenceTemplateError> {
    if schema != SCHEMA_VERSION {
        return Err(PriorEvidenceTemplateError::UnsupportedSchema(schema));
    }
    if semantic != SEMANTIC_VERSION {
        return Err(PriorEvidenceTemplateError::UnsupportedSemantics(semantic));
    }
    Ok(())
}
