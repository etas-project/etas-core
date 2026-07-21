#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition<State, Label> {
    pub from: State,
    pub to: State,
    pub label: Label,
}

impl<State, Label> Transition<State, Label> {
    pub fn new(from: State, to: State, label: Label) -> Self {
        Self { from, to, label }
    }
}
