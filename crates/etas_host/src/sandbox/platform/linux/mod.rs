mod syscalls;

use super::IsolationRequirements;
use crate::{HostError, HostErrorCode, SandboxPolicy};
use landlock::{
    ABI, Access, AccessFs, CompatLevel, Compatible, PathBeneath, Ruleset, RulesetAttr,
    RulesetCreatedAttr,
};
use std::{
    fs::OpenOptions,
    io,
    os::fd::{AsFd, AsRawFd, OwnedFd},
    os::unix::fs::OpenOptionsExt,
};

pub(crate) struct PreparedLandlock {
    ruleset: OwnedFd,
    filter: syscalls::Filter,
    requested: IsolationRequirements,
}

impl PreparedLandlock {
    pub(crate) fn prepare(
        policy: &SandboxPolicy,
        requested: IsolationRequirements,
    ) -> Result<Self, HostError> {
        // This backend intentionally offers offline native execution, not a
        // port-based approximation of endpoint authority.
        if !policy.network.allowed_endpoints.is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "landlock command isolation supports deny-all network only; use a host adapter for endpoint-authorized networking",
            ));
        }
        for program in &policy.command.allowed_programs {
            if !std::path::Path::new(program).is_absolute() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "isolated command allowlist requires absolute executable paths",
                )
                .with_detail("program", program));
            }
        }
        let mut ruleset = Ruleset::default()
            .set_compatibility(CompatLevel::HardRequirement)
            .handle_access(AccessFs::from_all(ABI::V5))
            .map_err(backend_error)?
            .create()
            .map_err(backend_error)?;
        for root in &policy.filesystem.read_roots {
            ruleset = ruleset
                .add_rule(PathBeneath::new(
                    root.directory().as_fd(),
                    AccessFs::ReadFile | AccessFs::ReadDir,
                ))
                .map_err(backend_error)?;
        }
        for root in &policy.filesystem.write_roots {
            ruleset = ruleset
                .add_rule(PathBeneath::new(
                    root.directory().as_fd(),
                    AccessFs::WriteFile
                        | AccessFs::Truncate
                        | AccessFs::MakeReg
                        | AccessFs::MakeDir
                        | AccessFs::MakeSym
                        | AccessFs::Refer,
                ))
                .map_err(backend_error)?;
        }
        if policy.destructive_ops.allow_delete {
            for root in &policy.filesystem.delete_roots {
                ruleset = ruleset
                    .add_rule(PathBeneath::new(
                        root.directory().as_fd(),
                        AccessFs::RemoveFile | AccessFs::RemoveDir,
                    ))
                    .map_err(backend_error)?;
            }
        }
        // Authorize executable objects, never an entire executable directory or
        // an implicit PATH. The exec itself is checked against these same inodes.
        for program in &policy.command.allowed_programs {
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_PATH | libc::O_CLOEXEC)
                .open(program)
                .map_err(backend_error)?;
            if !file.metadata().map_err(backend_error)?.is_file() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "isolated command target is not a regular file",
                ));
            }
            ruleset = ruleset
                .add_rule(PathBeneath::new(
                    file.as_fd(),
                    AccessFs::Execute | AccessFs::ReadFile,
                ))
                .map_err(backend_error)?;
        }
        let ruleset: Option<OwnedFd> = ruleset.into();
        let ruleset = ruleset
            .ok_or_else(|| backend_error("strict Landlock ruleset has no kernel descriptor"))?;
        Ok(Self {
            ruleset,
            filter: syscalls::Filter::new(),
            requested,
        })
    }

    pub(crate) fn requested(&self) -> IsolationRequirements {
        self.requested
    }

    pub(crate) fn install(mut self, command: &mut tokio::process::Command) {
        // All allocations/rule construction happen in the parent. Only raw
        // syscalls and stack data are used after fork, before untrusted exec.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::syscall(
                    libc::SYS_landlock_restrict_self,
                    self.ruleset.as_raw_fd(),
                    0,
                ) != 0
                {
                    return Err(io::Error::last_os_error());
                }
                self.filter.install()
            });
        }
    }
}

fn backend_error(error: impl std::fmt::Display) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "cannot prepare required Landlock ABI v5 isolation",
    )
    .with_detail("backend", "landlock")
    .with_detail("error", error.to_string())
}
