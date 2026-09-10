use super::{CommandIsolation, CommandIsolationReport};
use crate::{HostError, HostErrorCode, SandboxPolicy};
use tokio::process::Command;

pub(crate) enum PreparedIsolation {
    TrustedUnconfined,
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ))]
    Landlock(super::linux::PreparedLandlock),
}

impl PreparedIsolation {
    pub(crate) fn prepare(policy: &SandboxPolicy) -> Result<Self, HostError> {
        match policy.command.isolation {
            CommandIsolation::Denied => Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "native command execution is not authorized",
            )),
            CommandIsolation::TrustedUnconfined => Ok(Self::TrustedUnconfined),
            #[cfg(all(
                target_os = "linux",
                any(target_arch = "aarch64", target_arch = "x86_64")
            ))]
            CommandIsolation::Required {
                backend: super::PlatformSandboxHook::Landlock,
                guarantees,
            } => super::linux::PreparedLandlock::prepare(policy, guarantees).map(Self::Landlock),
            CommandIsolation::Required { backend, .. } => Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "requested isolation backend is unavailable on this platform",
            )
            .with_detail("backend", backend.name())
            .with_detail("platform", std::env::consts::OS)),
        }
    }

    pub(crate) fn install(self, command: &mut Command) -> PendingActivation {
        match self {
            Self::TrustedUnconfined => {
                let _ = command;
                PendingActivation {
                    kind: ActivatedKind::TrustedUnconfined,
                }
            }
            #[cfg(all(
                target_os = "linux",
                any(target_arch = "aarch64", target_arch = "x86_64")
            ))]
            Self::Landlock(prepared) => {
                let requested = prepared.requested();
                prepared.install(command);
                PendingActivation {
                    kind: ActivatedKind::Landlock(requested),
                }
            }
        }
    }
}

// This receipt cannot be constructed by callers. A successful std::process spawn
// acknowledges all pre_exec hooks through the CLOEXEC error pipe.
pub(crate) struct PendingActivation {
    kind: ActivatedKind,
}
enum ActivatedKind {
    TrustedUnconfined,
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "aarch64", target_arch = "x86_64")
    ))]
    Landlock(super::IsolationRequirements),
}
impl PendingActivation {
    pub(crate) fn after_spawn(self) -> CommandIsolationReport {
        match self.kind {
            ActivatedKind::TrustedUnconfined => CommandIsolationReport::trusted_unconfined(),
            #[cfg(all(
                target_os = "linux",
                any(target_arch = "aarch64", target_arch = "x86_64")
            ))]
            ActivatedKind::Landlock(requested) => CommandIsolationReport::landlock(requested),
        }
    }
}
