use super::{Matcher, NoTransitionPolicy, State as AutomatonState, StepResult, SymbolicAutomaton};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutomatonMonitor<State, Label> {
    automaton: SymbolicAutomaton<State, Label>,
}

impl<State, Label> AutomatonMonitor<State, Label>
where
    State: AutomatonState,
{
    pub fn new(automaton: SymbolicAutomaton<State, Label>) -> Self {
        Self { automaton }
    }

    pub fn with_no_transition_policy(initial: State, policy: NoTransitionPolicy) -> Self {
        Self::new(SymbolicAutomaton::new(initial, policy))
    }

    pub fn automaton(&self) -> &SymbolicAutomaton<State, Label> {
        &self.automaton
    }

    pub fn initial(&self) -> &State {
        self.automaton.initial()
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
        self.automaton.step(state, event, matcher)
    }
}
