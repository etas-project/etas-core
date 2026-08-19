#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostRequestKind {
    Model,
    Tool,
    Approval,
    Memory,
    Session,
    Console,
    Tcp,
    Stream,
    Tls,
    Filesystem,
    Secret,
    Browser,
    Command,
    Policy,
}
