use super::{command::CommandPolicy, filesystem::FilesystemPolicy, network::NetworkPolicy};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxPolicy {
    pub mode: SandboxMode,
    pub filesystem: FilesystemPolicy,
    pub network: NetworkPolicy,
    pub command: CommandPolicy,
    pub destructive_ops: DestructiveOpPolicy,
}

impl SandboxPolicy {
    pub fn deny_all() -> Self {
        Self {
            mode: SandboxMode::DenyAll,
            filesystem: FilesystemPolicy::deny_all(),
            network: NetworkPolicy::deny_all(),
            command: CommandPolicy::deny_all(),
            destructive_ops: DestructiveOpPolicy::deny_all(),
        }
    }

    pub fn allow_listed(
        filesystem: FilesystemPolicy,
        network: NetworkPolicy,
        command: CommandPolicy,
        destructive_ops: DestructiveOpPolicy,
    ) -> Self {
        Self {
            mode: SandboxMode::AllowListed,
            filesystem,
            network,
            command,
            destructive_ops,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SandboxMode {
    DenyAll,
    AllowListed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestructiveOpPolicy {
    pub allow_delete: bool,
}

impl DestructiveOpPolicy {
    pub fn deny_all() -> Self {
        Self {
            allow_delete: false,
        }
    }

    pub fn allow_workspace_delete() -> Self {
        Self { allow_delete: true }
    }
}
