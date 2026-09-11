use super::{IsolationRequirements, PlatformSandboxHook};

/// Only backend integration can construct evidence of activated restrictions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandIsolationReport {
    platform: &'static str,
    backend: Option<PlatformSandboxHook>,
    requested: IsolationRequirements,
    active: IsolationRequirements,
}
impl CommandIsolationReport {
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ))]
    pub(super) fn landlock(requested: IsolationRequirements) -> Self {
        Self {
            platform: "linux",
            backend: Some(PlatformSandboxHook::Landlock),
            requested,
            active: IsolationRequirements::all(),
        }
    }
    pub fn platform(&self) -> &'static str {
        self.platform
    }
    pub fn backend(&self) -> Option<PlatformSandboxHook> {
        self.backend
    }
    pub fn requested(&self) -> IsolationRequirements {
        self.requested
    }
    pub fn active(&self) -> IsolationRequirements {
        self.active
    }
    pub fn trusted_unconfined() -> Self {
        Self {
            platform: std::env::consts::OS,
            backend: None,
            requested: IsolationRequirements::default(),
            active: IsolationRequirements::default(),
        }
    }
}
