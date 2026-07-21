#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRequestKind {
    Model,
    Tool,
    Approval,
    Memory,
    Console,
    Network,
    Filesystem,
    Command,
    Policy,
}
