#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeState {
    Running,
    Draining,
    Stopping,
    Terminated,
}
