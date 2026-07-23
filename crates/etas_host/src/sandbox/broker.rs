use std::path::Path;

use super::filesystem::WorkspaceFileMetadata;

use crate::{
    CommandSandbox, DestructiveOpPolicy, FilesystemSandbox, HostError, HostErrorCode,
    NetworkSandbox, SandboxMode, SandboxPolicy, WorkspaceDiff, WorkspacePath, WorkspaceRoot,
    WorkspaceSnapshot,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxBroker {
    policy: SandboxPolicy,
    filesystem: FilesystemSandbox,
    network: NetworkSandbox,
    command: CommandSandbox,
}

impl SandboxBroker {
    pub fn new(policy: SandboxPolicy) -> Self {
        Self {
            filesystem: FilesystemSandbox::new(policy.filesystem.clone()),
            network: NetworkSandbox::new(policy.network.clone()),
            command: CommandSandbox::new(policy.command.clone()),
            policy,
        }
    }

    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    pub fn read_file(&self, root: &WorkspaceRoot, path: &Path) -> Result<Vec<u8>, HostError> {
        self.ensure_not_deny_all()?;
        self.filesystem.read_file(root, path)
    }

    pub fn atomic_write(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
        bytes: &[u8],
    ) -> Result<WorkspacePath, HostError> {
        self.ensure_not_deny_all()?;
        self.filesystem.atomic_write(root, path, bytes)
    }

    pub fn create_dir_all(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspacePath, HostError> {
        self.ensure_not_deny_all()?;
        self.filesystem.create_dir_all(root, path)
    }

    pub fn read_dir(&self, root: &WorkspaceRoot, path: &Path) -> Result<Vec<String>, HostError> {
        self.ensure_not_deny_all()?;
        self.filesystem.read_dir(root, path)
    }

    pub fn stat(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspaceFileMetadata, HostError> {
        self.ensure_not_deny_all()?;
        self.filesystem.stat(root, path)
    }

    pub fn delete_file(
        &self,
        root: &WorkspaceRoot,
        path: &Path,
    ) -> Result<WorkspacePath, HostError> {
        self.ensure_destructive_ops_allowed(&self.policy.destructive_ops)?;
        self.filesystem.delete_file(root, path)
    }

    pub fn check_network_endpoint(
        &self,
        scheme: &str,
        host: &str,
        port: u16,
    ) -> Result<(), HostError> {
        self.ensure_not_deny_all()?;
        self.network.check_endpoint(scheme, host, port)
    }

    pub fn resolve_network_endpoint(
        &self,
        scheme: &str,
        host: &str,
        port: u16,
    ) -> Result<Vec<std::net::SocketAddr>, HostError> {
        self.ensure_not_deny_all()?;
        self.network.resolve_endpoint(scheme, host, port)
    }

    pub(crate) fn validate_resolved_network_addresses(
        &self,
        scheme: &str,
        host: &str,
        port: u16,
        addresses: impl IntoIterator<Item = std::net::SocketAddr>,
    ) -> Result<Vec<std::net::SocketAddr>, HostError> {
        self.ensure_not_deny_all()?;
        self.network
            .validate_resolved_addresses(scheme, host, port, addresses)
    }

    pub fn check_command(&self, program: &str) -> Result<(), HostError> {
        self.ensure_not_deny_all()?;
        self.command.check_program(program)
    }

    pub fn snapshot(&self, root: WorkspaceRoot) -> Result<WorkspaceSnapshot, HostError> {
        self.ensure_not_deny_all()?;
        WorkspaceSnapshot::capture(root)
    }

    pub fn diff(&self, snapshot: &WorkspaceSnapshot) -> Result<WorkspaceDiff, HostError> {
        self.ensure_not_deny_all()?;
        snapshot.diff_current()
    }

    pub fn rollback(&self, snapshot: &WorkspaceSnapshot) -> Result<WorkspaceDiff, HostError> {
        self.ensure_not_deny_all()?;
        snapshot.rollback()
    }

    fn ensure_not_deny_all(&self) -> Result<(), HostError> {
        match self.policy.mode {
            SandboxMode::DenyAll => Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "sandbox policy denies all host operations",
            )),
            SandboxMode::AllowListed => Ok(()),
        }
    }

    fn ensure_destructive_ops_allowed(
        &self,
        policy: &DestructiveOpPolicy,
    ) -> Result<(), HostError> {
        self.ensure_not_deny_all()?;
        if policy.allow_delete {
            Ok(())
        } else {
            Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "destructive workspace operations are not allowed",
            ))
        }
    }
}
