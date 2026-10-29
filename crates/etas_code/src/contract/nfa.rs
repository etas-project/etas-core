use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::descriptor::{
    DescriptorGraph, EffectInfo, EffectRowInfoRef, EffectTargetInfo, ParameterCallInfo,
    ParameterHandlerScopeInfo, StaticEffectArgInfo, TraceInfo, TraceInfoRef,
};

use super::labels::{callable_work, key_work};
use super::{ProofBudget, TraceLabel, TraceProofError, canonical_effect};

pub(super) struct Edge {
    pub label: Option<CompiledLabel>,
    pub to: usize,
    pub source: TraceInfoRef,
}

#[derive(Clone)]
pub(super) struct CompiledLabel {
    pub value: Arc<TraceLabel>,
    pub comparison_work: usize,
}

impl CompiledLabel {
    fn new(label: TraceLabel) -> Self {
        let comparison_work = label.comparison_work();
        Self {
            value: Arc::new(label),
            comparison_work,
        }
    }
}

pub(super) struct Nfa {
    pub edges: Vec<Vec<Edge>>,
    pub start: usize,
    pub end: usize,
    transitions: usize,
    labels: BTreeMap<LabelSource, CompiledLabel>,
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
                        if let crate::descriptor::CallableOutcomeInfo::ParameterNonReturn(call) =
                            outcome
                        {
                            charge_call(graph, call, budget)?;
                        } else {
                            charge_graph(graph, budget)?;
                            if let crate::descriptor::CallableOutcomeInfo::Returned {
                                observation:
                                    crate::descriptor::ReturnObservationInfo::Variant { owner, name },
                                ..
                            } = outcome
                            {
                                budget.charge(key_work(owner).saturating_add(name.len()))?;
                            }
                        }
                        let label =
                            CompiledLabel::new(super::labels::outcome_label(graph, outcome)?);
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
        label: Option<CompiledLabel>,
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
    ) -> Result<CompiledLabel, TraceProofError> {
        budget.charge(1)?;
        if let Some(label) = self.labels.get(&source) {
            return Ok(label.clone());
        }
        charge_effect(graph, effect, budget)?;
        let label = CompiledLabel::new(TraceLabel::Action(canonical_effect(graph, effect)?));
        self.labels.insert(source, label.clone());
        Ok(label)
    }

    fn call_label(
        &mut self,
        graph: &DescriptorGraph,
        source: LabelSource,
        call: &ParameterCallInfo,
        budget: &mut ProofBudget,
    ) -> Result<CompiledLabel, TraceProofError> {
        budget.charge(1)?;
        if let Some(label) = self.labels.get(&source) {
            return Ok(label.clone());
        }
        charge_call(graph, call, budget)?;
        let label = CompiledLabel::new(TraceLabel::ParameterCall {
            callable: call.callable.clone(),
            position: call.position,
            signature: graph.type_fingerprint(call.signature)?,
            arguments: call.arguments.clone(),
            handlers: super::labels::handler_scope_labels(graph, call)?,
        });
        self.labels.insert(source, label.clone());
        Ok(label)
    }
}

fn charge_effect(
    graph: &DescriptorGraph,
    effect: &EffectInfo,
    budget: &mut ProofBudget,
) -> Result<(), TraceProofError> {
    let target = match &effect.target {
        EffectTargetInfo::Effect(key) => key_work(key),
        EffectTargetInfo::Action { effect, name } => key_work(effect).saturating_add(name.len()),
    };
    budget.charge(target)?;
    budget.charge(effect.arguments.len().saturating_add(1))?;
    for argument in &effect.arguments {
        match argument {
            StaticEffectArgInfo::Type(_) => charge_graph(graph, budget)?,
            StaticEffectArgInfo::Resource(key) => budget.charge(key_work(key))?,
            StaticEffectArgInfo::Callable(id) => budget.charge(callable_work(id))?,
            StaticEffectArgInfo::String(value) | StaticEffectArgInfo::Integer(value) => {
                budget.charge(value.len())?
            }
            StaticEffectArgInfo::Wildcard => {}
        }
    }
    Ok(())
}

fn charge_call(
    graph: &DescriptorGraph,
    call: &ParameterCallInfo,
    budget: &mut ProofBudget,
) -> Result<(), TraceProofError> {
    budget.charge(callable_work(&call.callable))?;
    charge_graph(graph, budget)?;
    budget.charge(call.arguments.as_ref().map_or(0, Vec::len))?;
    for handler in call.handlers.iter().flatten() {
        let ParameterHandlerScopeInfo::Resume { arms } = handler;
        budget.charge(arms.len().saturating_add(1))?;
        for arm in arms {
            charge_graph(graph, budget)?;
            charge_effect(graph, &arm.action, budget)?;
        }
    }
    Ok(())
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
