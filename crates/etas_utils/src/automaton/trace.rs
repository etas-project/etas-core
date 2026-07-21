#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceStep<State, Label> {
    pub from: State,
    pub to: State,
    pub label: Label,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductTraceStep<Component, State, Label> {
    pub component: Component,
    pub step: TraceStep<State, Label>,
}
