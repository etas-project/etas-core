use std::collections::BTreeSet;

pub trait State: Clone + Ord {}

impl<T> State for T where T: Clone + Ord {}

pub type StateSet<S> = BTreeSet<S>;
pub type InitialState<S> = S;
pub type AcceptingState<S> = S;
pub type RejectingState<S> = S;
