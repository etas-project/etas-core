mod inclusion;
mod labels;
mod nfa;

use crate::descriptor::DescriptorError;

pub use inclusion::{TraceInclusion, TraceInclusionWitness, check_trace_inclusion};
pub use labels::{CanonicalEffect, CanonicalSelector, TraceLabel, canonical_effect};

#[derive(Clone, Copy, Debug)]
pub struct TraceProofLimits {
    pub max_automaton_states: usize,
    pub max_transitions: usize,
    pub max_product_states: usize,
    pub max_product_members: usize,
    pub max_steps: usize,
}

impl Default for TraceProofLimits {
    fn default() -> Self {
        Self {
            max_automaton_states: 8_192,
            max_transitions: 65_536,
            max_product_states: 8_192,
            max_product_members: 262_144,
            max_steps: 1_048_576,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceProofError {
    Descriptor(DescriptorError),
    MissingTrace(crate::descriptor::TraceInfoRef),
    MissingRow(crate::descriptor::EffectRowInfoRef),
    Limit {
        resource: &'static str,
        limit: usize,
    },
}

pub(super) struct ProofBudget {
    limits: TraceProofLimits,
    steps: usize,
}

impl ProofBudget {
    fn charge(&mut self, count: usize) -> Result<(), TraceProofError> {
        if count > self.limits.max_steps.saturating_sub(self.steps) {
            return Err(TraceProofError::Limit {
                resource: "proof steps",
                limit: self.limits.max_steps,
            });
        }
        self.steps += count;
        Ok(())
    }
}

impl From<DescriptorError> for TraceProofError {
    fn from(error: DescriptorError) -> Self {
        Self::Descriptor(error)
    }
}

impl std::fmt::Display for TraceProofError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "trace inclusion proof failed: {self:?}")
    }
}
impl std::error::Error for TraceProofError {}
