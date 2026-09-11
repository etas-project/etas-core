use crate::HostError;
use tokio::process::Command;

#[cfg(unix)]
pub(super) fn configure(command: &mut Command) -> Result<(), HostError> {
    // The single-threaded post-fork child is the only safe place to enumerate
    // inheritable descriptors without races with other host services. Preserve
    // setup descriptors until exec (including Rust's spawn-error pipe).
    unsafe {
        command.pre_exec(|| {
            for fd in close_fds::iter_open_fds(3) {
                if libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn configure(_command: &mut Command) -> Result<(), HostError> {
    Err(HostError::new(
        crate::HostErrorCode::ProviderUnavailable,
        "explicit command handle inheritance allowlist is unavailable on this platform",
    ))
}
