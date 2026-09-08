use tokio::process::{Child, Command};

use crate::{HostError, HostErrorCode};

#[derive(Clone, Copy, Debug)]
pub(super) struct ProcessTreeController {
    #[cfg(unix)]
    process_group: libc::pid_t,
}

impl ProcessTreeController {
    pub(super) fn configure(command: &mut Command) {
        command.kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
    }

    pub(super) fn for_child(child: &Child, program: &str) -> Result<Self, HostError> {
        let process_id = child.id().ok_or_else(|| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "spawned command has no process identity",
            )
            .with_detail("program", program.to_owned())
        })?;
        #[cfg(unix)]
        {
            let process_group = libc::pid_t::try_from(process_id).map_err(|_| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "spawned command process identity is outside the platform range",
                )
                .with_detail("program", program.to_owned())
                .with_detail("process_id", process_id.to_string())
            })?;
            Ok(Self { process_group })
        }
        #[cfg(not(unix))]
        {
            let _ = process_id;
            Ok(Self {})
        }
    }

    pub(super) fn kill(&self, child: &mut Child) -> Result<(), HostError> {
        #[cfg(unix)]
        {
            let _ = child;
            self.signal_group(libc::SIGKILL)
        }
        #[cfg(not(unix))]
        {
            child.start_kill().map_err(process_control_error)
        }
    }

    pub(super) fn kill_from_drop(&self) {
        #[cfg(unix)]
        {
            let _ = self.signal_group(libc::SIGKILL);
        }
    }

    #[cfg(unix)]
    fn signal_group(&self, signal: libc::c_int) -> Result<(), HostError> {
        // SAFETY: the child was placed in a process group whose ID is its validated PID.
        let result = unsafe { libc::kill(-self.process_group, signal) };
        if result == 0 {
            return Ok(());
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        Err(process_control_error(error))
    }
}

fn process_control_error(error: std::io::Error) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "failed to control command process tree",
    )
    .with_detail("error", error.to_string())
}
