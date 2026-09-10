mod config;
#[cfg(test)]
mod tests;
mod transaction;
mod worker;
use crate::execution::DispatchError;
use crate::{ExecutionBudget, HostError, execution::OperationContext};
pub(crate) use config::open_durable;
pub(crate) use transaction::write_transaction;
pub(crate) use worker::SqliteWorker;

#[derive(Clone, Debug)]
pub(crate) struct SqliteOperation {
    pub context: OperationContext,
    pub budget: ExecutionBudget,
}
impl SqliteOperation {
    pub fn check(&self) -> Result<(), HostError> {
        self.context.signal().check()?;
        self.budget.check_time()
    }
}
