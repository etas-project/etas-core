mod identity;
mod registry;
mod state;

pub use identity::ScopeId;
pub use registry::ExecutionScope;
pub(crate) use registry::invalid_state;
pub use state::ScopeState;
