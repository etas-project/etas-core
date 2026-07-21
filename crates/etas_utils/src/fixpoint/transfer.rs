pub trait Transfer<State> {
    fn apply(&self, state: &mut State) -> bool;
}

impl<State, F> Transfer<State> for F
where
    F: Fn(&mut State) -> bool,
{
    fn apply(&self, state: &mut State) -> bool {
        self(state)
    }
}

pub trait NodeTransfer<Node, State> {
    fn apply_node(&self, node: &Node, state: &mut State) -> bool;
}

impl<Node, State, F> NodeTransfer<Node, State> for F
where
    F: Fn(&Node, &mut State) -> bool,
{
    fn apply_node(&self, node: &Node, state: &mut State) -> bool {
        self(node, state)
    }
}

pub trait EdgeTransfer<Node, State> {
    fn apply_edge(&self, from: &Node, to: &Node, state: &mut State) -> bool;
}

impl<Node, State, F> EdgeTransfer<Node, State> for F
where
    F: Fn(&Node, &Node, &mut State) -> bool,
{
    fn apply_edge(&self, from: &Node, to: &Node, state: &mut State) -> bool {
        self(from, to, state)
    }
}

pub trait Constraint<State> {
    fn apply_constraint(&self, state: &mut State) -> bool;
}

impl<State, F> Constraint<State> for F
where
    F: Fn(&mut State) -> bool,
{
    fn apply_constraint(&self, state: &mut State) -> bool {
        self(state)
    }
}
