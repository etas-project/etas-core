use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::descriptor::{
    DescriptorGraph, EffectInfo, EffectRowInfoRef, ParameterCallInfo, TraceInfo, TraceInfoRef,
};

use super::{ProofBudget, TraceLabel, TraceProofError, canonical_effect};

pub(super) struct Edge {
    pub label: Option<Arc<TraceLabel>>,
    pub to: usize,
    pub source: TraceInfoRef,
}

pub(super) struct Nfa {
    pub edges: Vec<Vec<Edge>>,
    pub start: usize,
    pub end: usize,
    transitions: usize,
    labels: BTreeMap<LabelSource, Arc<TraceLabel>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LabelSource {
    Trace(TraceInfoRef),
    Row(EffectRowInfoRef, usize),
    Parameter(TraceInfoRef, usize),
}

impl Nfa {
    pub fn build(
        graph: &DescriptorGraph,
        root: TraceInfoRef,
        budget: &mut ProofBudget,
    ) -> Result<Self, TraceProofError> {
        let mut result = Self {
            edges: Vec::new(),
            start: 0,
            end: 0,
            transitions: 0,
            labels: BTreeMap::new(),
        };
        result.start = result.state(budget)?;
        result.end = result.state(budget)?;
        let mut work = vec![(root, result.start, result.end)];
        while let Some((source, start, end)) = work.pop() {
            budget.charge(1)?;
            match graph
                .trace(source)
                .ok_or(TraceProofError::MissingTrace(source))?
            {
                TraceInfo::Empty => result.edge(start, end, None, source, budget)?,
                TraceInfo::Outcome(outcome) => {
                    let key = LabelSource::Trace(source);
                    let label = if let Some(label) = result.labels.get(&key) {
                        label.clone()
                    } else {
                        charge_graph(graph, budget)?;
                        let label = Arc::new(super::labels::outcome_label(graph, outcome)?);
                        result.labels.insert(key, label.clone());
                        label
                    };
                    result.edge(start, end, Some(label), source, budget)?;
                }
                TraceInfo::Event(event) => {
                    let label =
                        result.action_label(graph, LabelSource::Trace(source), event, budget)?;
                    result.edge(start, end, Some(label), source, budget)?;
                }
                TraceInfo::ParameterCall(call) => {
                    let label =
                        result.call_label(graph, LabelSource::Trace(source), call, budget)?;
                    result.edge(start, end, Some(label), source, budget)?;
                }
                TraceInfo::Seq(parts) => {
                    budget.charge(parts.len())?;
                    if parts.is_empty() {
                        result.edge(start, end, None, source, budget)?;
                    }
                    let mut previous = start;
                    for (index, part) in parts.iter().enumerate() {
                        let next = if index + 1 == parts.len() {
                            end
                        } else {
                            result.state(budget)?
                        };
                        work.push((*part, previous, next));
                        previous = next;
                    }
                }
                TraceInfo::Choice(parts) => {
                    budget.charge(parts.len())?;
                    // Empty Choice denotes the empty language, unlike Empty.
                    work.extend(parts.iter().map(|part| (*part, start, end)));
                }
                TraceInfo::Repeat(inner) => {
                    let inner_start = result.state(budget)?;
                    let inner_end = result.state(budget)?;
                    result.edge(start, end, None, source, budget)?;
                    result.edge(start, inner_start, None, source, budget)?;
                    result.edge(inner_end, inner_start, None, source, budget)?;
                    result.edge(inner_end, end, None, source, budget)?;
                    work.push((*inner, inner_start, inner_end));
                }
                TraceInfo::UnknownOrder(actions) | TraceInfo::Widened { actions, .. } => {
                    let row = graph
                        .effect_row(*actions)
                        .ok_or(TraceProofError::MissingRow(*actions))?;
                    for (index, event) in row.effects.iter().enumerate() {
                        let label = result.action_label(
                            graph,
                            LabelSource::Row(*actions, index),
                            event,
                            budget,
                        )?;
                        result.edge(start, start, Some(label), source, budget)?;
                    }
                    if let Some(TraceInfo::Widened {
                        parameter_calls, ..
                    }) = graph.trace(source)
                    {
                        for (index, call) in parameter_calls.iter().enumerate() {
                            let label = result.call_label(
                                graph,
                                LabelSource::Parameter(source, index),
                                call,
                                budget,
                            )?;
                            result.edge(start, start, Some(label), source, budget)?;
                        }
                    }
                    result.edge(start, end, None, source, budget)?;
                }
            }
        }
        Ok(result)
    }

    fn state(&mut self, budget: &mut ProofBudget) -> Result<usize, TraceProofError> {
        budget.charge(1)?;
        if self.edges.len() >= budget.limits.max_automaton_states {
            return Err(TraceProofError::Limit {
                resource: "automaton states",
                limit: budget.limits.max_automaton_states,
            });
        }
        let state = self.edges.len();
        self.edges.push(Vec::new());
        Ok(state)
    }

    fn edge(
        &mut self,
        from: usize,
        to: usize,
        label: Option<Arc<TraceLabel>>,
        source: TraceInfoRef,
        budget: &mut ProofBudget,
    ) -> Result<(), TraceProofError> {
        budget.charge(1)?;
        if self.transitions >= budget.limits.max_transitions {
            return Err(TraceProofError::Limit {
                resource: "automaton transitions",
                limit: budget.limits.max_transitions,
            });
        }
        self.transitions += 1;
        self.edges[from].push(Edge { label, to, source });
        Ok(())
    }

    pub fn closure(
        &self,
        states: impl IntoIterator<Item = usize>,
        budget: &mut ProofBudget,
    ) -> Result<BTreeSet<usize>, TraceProofError> {
        let mut visited = BTreeSet::new();
        let mut pending = states.into_iter().collect::<Vec<_>>();
        while let Some(state) = pending.pop() {
            budget.charge(1)?;
            if !visited.insert(state) {
                continue;
            }
            for edge in &self.edges[state] {
                budget.charge(1)?;
                if edge.label.is_none() {
                    pending.push(edge.to);
                }
            }
        }
        Ok(visited)
    }

    fn action_label(
        &mut self,
        graph: &DescriptorGraph,
        source: LabelSource,
        effect: &EffectInfo,
        budget: &mut ProofBudget,
    ) -> Result<Arc<TraceLabel>, TraceProofError> {
        budget.charge(1)?;
        if let Some(label) = self.labels.get(&source) {
            return Ok(label.clone());
        }
        charge_graph(graph, budget)?;
        let label = Arc::new(TraceLabel::Action(canonical_effect(graph, effect)?));
        self.labels.insert(source, label.clone());
        Ok(label)
    }

    fn call_label(
        &mut self,
        graph: &DescriptorGraph,
        source: LabelSource,
        call: &ParameterCallInfo,
        budget: &mut ProofBudget,
    ) -> Result<Arc<TraceLabel>, TraceProofError> {
        budget.charge(1)?;
        if let Some(label) = self.labels.get(&source) {
            return Ok(label.clone());
        }
        charge_graph(graph, budget)?;
        let label = Arc::new(TraceLabel::ParameterCall {
            callable: call.callable.clone(),
            position: call.position,
            signature: graph.type_fingerprint(call.signature)?,
            arguments: call.arguments.clone(),
        });
        self.labels.insert(source, label.clone());
        Ok(label)
    }
}

fn charge_graph(graph: &DescriptorGraph, budget: &mut ProofBudget) -> Result<(), TraceProofError> {
    let graph = graph.data();
    budget.charge(
        graph
            .types
            .len()
            .saturating_add(graph.rows.len())
            .saturating_add(graph.binders.len())
            .saturating_add(graph.traces.len()),
    )
}
