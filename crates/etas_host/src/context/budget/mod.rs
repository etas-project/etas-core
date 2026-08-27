pub mod cost;
mod execution;
pub mod time;
pub mod token;

pub use cost::CostBudget;
pub use execution::{
    CostReservation, ExecutionBudget, ExecutionBudgetSnapshot, ExecutionBudgetState,
    MonotonicClock, TokenReservation,
};
pub use time::TimeBudget;
pub use token::TokenBudget;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Budget {
    pub tokens: Option<TokenBudget>,
    pub time: Option<TimeBudget>,
    pub cost: Option<CostBudget>,
}
