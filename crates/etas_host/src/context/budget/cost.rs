#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostBudget {
    pub max_micros: u128,
    pub currency: String,
}
