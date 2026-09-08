use crate::{HostError, WorkspacePath};
use tokio::process::Command;

#[cfg(unix)]
pub(super) fn configure(command: &mut Command, cwd: &WorkspacePath) -> Result<(), HostError> {
    use std::os::fd::AsRawFd;
    let directory = cwd
        .root
        .directory()
        .open_dir(&cwd.relative)
        .map_err(crate::sandbox::workspace::workspace_io_error)?;
    // Keep the descriptor alive in the command until spawn. Only the child
    // changes cwd; no ambient parent-process cwd or pathname is consulted.
    unsafe {
        command.pre_exec(move || {
            if libc::fchdir(directory.as_raw_fd()) == 0 {
                Ok(())
            } else {
                Err(std::io::Error::last_os_error())
            }
        });
    }
    Ok(())
}

#[cfg(not(unix))]
pub(super) fn configure(_command: &mut Command, _cwd: &WorkspacePath) -> Result<(), HostError> {
    Err(HostError::new(
        crate::HostErrorCode::ProviderUnavailable,
        "descriptor-bound command cwd is unavailable on this platform",
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::TestWorkspace;
    use std::fs;

    #[test]
    fn child_cwd_keeps_directory_opened_before_replacement() {
        let fixture = TestWorkspace::create("command-cwd-race").unwrap();
        let outside = TestWorkspace::create("command-cwd-outside").unwrap();
        fs::create_dir(fixture.path().join("cwd")).unwrap();
        fs::write(fixture.path().join("cwd/data"), b"authorized").unwrap();
        fs::write(outside.path().join("data"), b"outside").unwrap();
        let cwd = fixture.root().unwrap().resolve_existing("cwd").unwrap();
        let mut command = Command::new("/bin/cat");
        command.arg("data");
        configure(&mut command, &cwd).unwrap();
        fs::rename(fixture.path().join("cwd"), fixture.path().join("retained")).unwrap();
        std::os::unix::fs::symlink(outside.path(), fixture.path().join("cwd")).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let output = runtime.block_on(async { command.output().await }).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"authorized");
    }
}
