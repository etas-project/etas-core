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
    pub fn conjoin(&self, restriction: &Self) -> Self {
        use super::platform::CommandIsolation;
        if self.mode == SandboxMode::DenyAll || restriction.mode == SandboxMode::DenyAll {
            return Self::deny_all();
        }
        let isolation = match (self.command.isolation, restriction.command.isolation) {
            (CommandIsolation::Denied, _) | (_, CommandIsolation::Denied) => {
                CommandIsolation::Denied
            }
            (CommandIsolation::TrustedUnconfined, other)
            | (other, CommandIsolation::TrustedUnconfined) => other,
            (
                CommandIsolation::Required {
                    backend,
                    guarantees: left,
                },
                CommandIsolation::Required {
                    backend: other,
                    guarantees: right,
                },
            ) if backend == other => CommandIsolation::Required {
                backend,
                guarantees: super::platform::IsolationRequirements {
                    filesystem: left.filesystem || right.filesystem,
                    network: left.network || right.network,
                    process: left.process || right.process,
                },
            },
            // No existing command backend can satisfy two different required
            // backend identities in one activation.
            _ => CommandIsolation::Denied,
        };
        Self::allow_listed(
            FilesystemPolicy {
                read_roots: intersection(
                    &self.filesystem.read_roots,
                    &restriction.filesystem.read_roots,
                ),
                write_roots: intersection(
                    &self.filesystem.write_roots,
                    &restriction.filesystem.write_roots,
                ),
                delete_roots: intersection(
                    &self.filesystem.delete_roots,
                    &restriction.filesystem.delete_roots,
                ),
            },
            NetworkPolicy::allow_endpoints(intersection(
                &self.network.allowed_endpoints,
                &restriction.network.allowed_endpoints,
            )),
            CommandPolicy {
                allowed_programs: intersection(
                    &self.command.allowed_programs,
                    &restriction.command.allowed_programs,
                ),
                isolation,
            },
            DestructiveOpPolicy {
                allow_delete: self.destructive_ops.allow_delete
                    && restriction.destructive_ops.allow_delete,
            },
        )
    }

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

fn intersection<T: Clone + PartialEq>(left: &[T], right: &[T]) -> Vec<T> {
    left.iter()
        .filter(|value| right.contains(value))
        .cloned()
        .collect()
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
