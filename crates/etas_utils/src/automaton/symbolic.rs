use std::collections::BTreeSet;

use super::{TraceStep, Transition};

pub trait Matcher<Label, Event> {
    fn matches(&self, label: &Label, event: &Event) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoTransitionPolicy {
    Stay,
    Reject,
    Empty,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepResult<State, Label> {
    pub states: BTreeSet<State>,
    pub accepting: BTreeSet<State>,
    pub rejecting: BTreeSet<State>,
    pub trace: Vec<TraceStep<State, Label>>,
}

impl<State, Label> StepResult<State, Label>
where
    State: Clone + Ord,
{
    pub fn singleton(state: State) -> Self {
        let states = BTreeSet::from([state]);
        Self {
            states,
            accepting: BTreeSet::new(),
            rejecting: BTreeSet::new(),
            trace: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolicAutomaton<State, Label> {
    initial: State,
    accepting: BTreeSet<State>,
    rejecting: BTreeSet<State>,
    transitions: Vec<Transition<State, Label>>,
    no_transition: NoTransitionPolicy,
}

impl<State, Label> SymbolicAutomaton<State, Label>
where
    State: Clone + Ord,
{
    pub fn new(initial: State, no_transition: NoTransitionPolicy) -> Self {
        Self {
            initial,
            accepting: BTreeSet::new(),
            rejecting: BTreeSet::new(),
            transitions: Vec::new(),
            no_transition,
        }
    }

    pub fn initial(&self) -> &State {
        &self.initial
    }

    pub fn accepting_states(&self) -> &BTreeSet<State> {
        &self.accepting
    }

    pub fn rejecting_states(&self) -> &BTreeSet<State> {
        &self.rejecting
    }

    pub fn transitions(&self) -> &[Transition<State, Label>] {
        &self.transitions
    }

    pub fn no_transition_policy(&self) -> NoTransitionPolicy {
        self.no_transition.clone()
    }

    pub fn add_transition(&mut self, from: State, to: State, label: Label) {
        self.transitions.push(Transition::new(from, to, label));
    }

    pub fn mark_accepting(&mut self, state: State) {
        self.accepting.insert(state);
    }

    pub fn mark_rejecting(&mut self, state: State) {
        self.rejecting.insert(state);
    }

    pub fn step<Event, M>(
        &self,
        state: &State,
        event: &Event,
        matcher: &M,
    ) -> StepResult<State, Label>
    where
        Label: Clone,
        M: Matcher<Label, Event>,
    {
        let mut result = StepResult {
            states: BTreeSet::new(),
            accepting: BTreeSet::new(),
            rejecting: BTreeSet::new(),
            trace: Vec::new(),
        };

        for transition in &self.transitions {
            if &transition.from != state || !matcher.matches(&transition.label, event) {
                continue;
            }
            result.states.insert(transition.to.clone());
            result.trace.push(TraceStep {
                from: transition.from.clone(),
                to: transition.to.clone(),
                label: transition.label.clone(),
            });
        }

        if result.states.is_empty() {
            match self.no_transition {
                NoTransitionPolicy::Stay => {
                    result.states.insert(state.clone());
                }
                NoTransitionPolicy::Reject => {
                    result.states.insert(state.clone());
                    result.rejecting.insert(state.clone());
                }
                NoTransitionPolicy::Empty => {}
            }
        }

        for state in &result.states {
            if self.accepting.contains(state) {
                result.accepting.insert(state.clone());
            }
            if self.rejecting.contains(state) {
                result.rejecting.insert(state.clone());
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::{Matcher, NoTransitionPolicy, SymbolicAutomaton};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct EqMatcher;

    impl Matcher<&'static str, &'static str> for EqMatcher {
        fn matches(&self, label: &&'static str, event: &&'static str) -> bool {
            label == event
        }
    }

    #[test]
    fn symbolic_step_uses_matcher_and_tracks_rejecting_state() {
        let mut automaton = SymbolicAutomaton::new("start", NoTransitionPolicy::Stay);
        automaton.add_transition("start", "rejected", "deny");
        automaton.mark_rejecting("rejected");

        let result = automaton.step(&"start", &"deny", &EqMatcher);

        assert!(result.states.contains("rejected"));
        assert!(result.rejecting.contains("rejected"));
        assert_eq!(result.trace.len(), 1);
    }

    #[test]
    fn symbolic_step_uses_explicit_stay_policy_when_no_transition_matches() {
        let mut automaton = SymbolicAutomaton::new("start", NoTransitionPolicy::Stay);
        automaton.add_transition("start", "next", "go");

        let result = automaton.step(&"start", &"stay", &EqMatcher);

        assert_eq!(result.states.into_iter().collect::<Vec<_>>(), vec!["start"]);
        assert!(result.trace.is_empty());
    }

    #[test]
    fn symbolic_step_can_reject_when_no_transition_matches() {
        let mut automaton = SymbolicAutomaton::new("start", NoTransitionPolicy::Reject);
        automaton.add_transition("start", "next", "go");

        let result = automaton.step(&"start", &"stay", &EqMatcher);

        assert_eq!(result.states.into_iter().collect::<Vec<_>>(), vec!["start"]);
        assert!(result.rejecting.contains("start"));
    }

    #[test]
    fn symbolic_step_can_return_empty_when_no_transition_matches() {
        let mut automaton = SymbolicAutomaton::new("start", NoTransitionPolicy::Empty);
        automaton.add_transition("start", "next", "go");

        let result = automaton.step(&"start", &"stay", &EqMatcher);

        assert!(result.states.is_empty());
        assert!(result.rejecting.is_empty());
    }
}
