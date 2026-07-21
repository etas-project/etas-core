pub mod monitor;
pub mod product;
pub mod state;
pub mod symbolic;
pub mod trace;
pub mod transition;

pub use monitor::AutomatonMonitor;
pub use product::{ProductAutomaton, ProductState, ProductStepResult, StepError};
pub use state::{AcceptingState, InitialState, RejectingState, State, StateSet};
pub use symbolic::{Matcher, NoTransitionPolicy, StepResult, SymbolicAutomaton};
pub use trace::{ProductTraceStep, TraceStep};
pub use transition::Transition;
