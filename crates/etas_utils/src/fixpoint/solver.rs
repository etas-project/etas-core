use std::hash::Hash;

use super::{ConvergenceStatus, FixpointResult, FixpointStats, IterationLimit, Transfer, Worklist};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FixpointEngine {
    limit: IterationLimit,
}

impl FixpointEngine {
    pub fn new(limit: IterationLimit) -> Self {
        Self { limit }
    }

    pub fn limit(&self) -> IterationLimit {
        self.limit
    }

    pub fn solve<State, Step>(&self, mut state: State, mut step: Step) -> FixpointResult<State>
    where
        Step: FnMut(&mut State) -> bool,
    {
        let mut stats = FixpointStats::default();

        while stats.iterations < self.limit.max_iterations() {
            stats.iterations += 1;
            if !step(&mut state) {
                return FixpointResult {
                    value: state,
                    status: ConvergenceStatus::Converged,
                    stats,
                };
            }
            stats.changes += 1;
        }

        FixpointResult {
            value: state,
            status: ConvergenceStatus::IterationLimitReached,
            stats,
        }
    }

    pub fn apply_transfers<State, T>(
        &self,
        state: State,
        transfers: impl IntoIterator<Item = T>,
    ) -> FixpointResult<State>
    where
        T: Transfer<State>,
    {
        let transfers = transfers.into_iter().collect::<Vec<_>>();
        self.solve(state, |state| {
            transfers
                .iter()
                .fold(false, |changed, transfer| transfer.apply(state) || changed)
        })
    }

    pub fn solve_worklist<Node, State, Step, Schedule>(
        &self,
        mut state: State,
        initial: impl IntoIterator<Item = Node>,
        mut step: Step,
        mut schedule: Schedule,
    ) -> FixpointResult<State>
    where
        Node: Clone + Eq + Hash,
        Step: FnMut(&Node, &mut State) -> bool,
        Schedule: FnMut(&Node, &State) -> Vec<Node>,
    {
        let mut stats = FixpointStats::default();
        let mut worklist = Worklist::from_iter(initial);

        while let Some(node) = worklist.pop() {
            if stats.iterations >= self.limit.max_iterations() {
                return FixpointResult {
                    value: state,
                    status: ConvergenceStatus::IterationLimitReached,
                    stats,
                };
            }

            stats.iterations += 1;
            if step(&node, &mut state) {
                stats.changes += 1;
                worklist.extend(schedule(&node, &state));
            }
        }

        FixpointResult {
            value: state,
            status: ConvergenceStatus::Converged,
            stats,
        }
    }
}
