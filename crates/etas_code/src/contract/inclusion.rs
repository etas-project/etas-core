use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::descriptor::{DescriptorGraph, TraceInfoRef};

use super::{ProofBudget, TraceLabel, TraceProofError, TraceProofLimits, nfa::Nfa};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, etas_core::RetainedSize)]
pub struct TraceInclusionWitness {
    /// An abstract counterexample, not necessarily a concrete execution witness.
    pub path: Vec<(TraceInfoRef, Arc<TraceLabel>)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceInclusion {
    Included,
    NotProven(TraceInclusionWitness),
}

struct SearchNode {
    candidate: usize,
    frozen: BTreeSet<usize>,
    parent: Option<(usize, TraceInfoRef, Arc<TraceLabel>)>,
}

/// Regular-language inclusion is stronger than equality of final monitor
/// states: it preserves every action prefix and hence intermediate violations.
/// Symbolic edges are usable only when they cover the entire candidate label.
pub fn check_trace_inclusion(
    candidate: &DescriptorGraph,
    candidate_root: TraceInfoRef,
    frozen: &DescriptorGraph,
    frozen_root: TraceInfoRef,
    limits: TraceProofLimits,
) -> Result<TraceInclusion, TraceProofError> {
    let mut budget = ProofBudget::new(limits, None);
    let candidate = Nfa::build(candidate, candidate_root, &mut budget)?;
    let frozen = Nfa::build(frozen, frozen_root, &mut budget)?;
    check_nfa_inclusion(&candidate, &frozen, &mut budget)
}

/// Prove that the non-return alternatives describe exactly the prefixes at
/// which a parameter call can occur, including repeated and widened traces.
pub fn check_parameter_exit_coverage(
    graph: &DescriptorGraph,
    trace: TraceInfoRef,
    outcomes: TraceInfoRef,
    limits: TraceProofLimits,
) -> Result<bool, TraceProofError> {
    parameter_exit_coverage(graph, trace, outcomes, limits, None)
}

pub fn check_parameter_exit_coverage_controlled(
    graph: &DescriptorGraph,
    trace: TraceInfoRef,
    outcomes: TraceInfoRef,
    limits: TraceProofLimits,
    control: &dyn etas_utils::PipelineExecutionControl,
) -> Result<bool, TraceProofError> {
    parameter_exit_coverage(graph, trace, outcomes, limits, Some(control))
}

fn parameter_exit_coverage(
    graph: &DescriptorGraph,
    trace: TraceInfoRef,
    outcomes: TraceInfoRef,
    limits: TraceProofLimits,
    control: Option<&dyn etas_utils::PipelineExecutionControl>,
) -> Result<bool, TraceProofError> {
    let mut budget = ProofBudget::new(limits, control);
    let expected = Nfa::build(graph, trace, &mut budget)?.parameter_exit_prefixes(&mut budget)?;
    let actual = Nfa::build(graph, outcomes, &mut budget)?.parameter_exit_outcomes(&mut budget)?;
    if !matches!(
        check_nfa_inclusion(&expected, &actual, &mut budget)?,
        TraceInclusion::Included
    ) {
        return Ok(false);
    }
    Ok(matches!(
        check_nfa_inclusion(&actual, &expected, &mut budget)?,
        TraceInclusion::Included
    ))
}

fn check_nfa_inclusion(
    candidate: &Nfa,
    frozen: &Nfa,
    budget: &mut ProofBudget,
) -> Result<TraceInclusion, TraceProofError> {
    let initial = frozen.closure([frozen.start], budget)?;
    let mut search = Vec::new();
    let mut visited = BTreeMap::new();
    let mut members = 0usize;
    enqueue(
        &mut search,
        &mut visited,
        &mut members,
        SearchNode {
            candidate: candidate.start,
            frozen: initial,
            parent: None,
        },
        budget,
    )?;
    let mut cursor = 0;
    while cursor < search.len() {
        budget.charge(1)?;
        let states = candidate.closure([search[cursor].candidate], budget)?;
        if states.contains(&candidate.end) && !search[cursor].frozen.contains(&frozen.end) {
            return Ok(TraceInclusion::NotProven(witness(&search, cursor, None)));
        }
        for state in states {
            for edge in &candidate.edges[state] {
                budget.charge(1)?;
                let Some(label) = &edge.label else {
                    continue;
                };
                let mut next = BTreeSet::new();
                for old in &search[cursor].frozen {
                    for old_edge in &frozen.edges[*old] {
                        budget.charge(1)?;
                        if let Some(bound) = &old_edge.label {
                            budget.charge(
                                bound.comparison_work.saturating_add(label.comparison_work),
                            )?;
                            if bound.value.covers(&label.value) {
                                next.insert(old_edge.to);
                            }
                        }
                    }
                }
                let next = frozen.closure(next, budget)?;
                if next.is_empty() {
                    return Ok(TraceInclusion::NotProven(witness(
                        &search,
                        cursor,
                        Some((edge.source, label.value.clone())),
                    )));
                }
                enqueue(
                    &mut search,
                    &mut visited,
                    &mut members,
                    SearchNode {
                        candidate: edge.to,
                        frozen: next,
                        parent: Some((cursor, edge.source, label.value.clone())),
                    },
                    budget,
                )?;
            }
        }
        cursor += 1;
    }
    Ok(TraceInclusion::Included)
}

fn witness(
    search: &[SearchNode],
    cursor: usize,
    last: Option<(TraceInfoRef, Arc<TraceLabel>)>,
) -> TraceInclusionWitness {
    let mut path = Vec::new();
    let mut parent = cursor;
    while let Some((previous, source, label)) = &search[parent].parent {
        path.push((*source, label.clone()));
        parent = *previous;
    }
    path.reverse();
    path.extend(last);
    TraceInclusionWitness { path }
}

fn enqueue(
    search: &mut Vec<SearchNode>,
    visited: &mut BTreeMap<(usize, BTreeSet<usize>), usize>,
    members: &mut usize,
    node: SearchNode,
    budget: &ProofBudget,
) -> Result<(), TraceProofError> {
    let key = (node.candidate, node.frozen.clone());
    if visited.contains_key(&key) {
        return Ok(());
    }
    if search.len() >= budget.limits.max_product_states {
        return Err(TraceProofError::Limit {
            resource: "product states",
            limit: budget.limits.max_product_states,
        });
    }
    if node.frozen.len() > budget.limits.max_product_members.saturating_sub(*members) {
        return Err(TraceProofError::Limit {
            resource: "product members",
            limit: budget.limits.max_product_members,
        });
    }
    *members += node.frozen.len();
    visited.insert(key, search.len());
    search.push(node);
    Ok(())
}
