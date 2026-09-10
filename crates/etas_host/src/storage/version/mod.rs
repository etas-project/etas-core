mod condition;
mod token;

pub use condition::WriteCondition;
pub use token::MemoryVersion;
pub(crate) use token::{StoreGeneration, next_revision, scope_identity};
