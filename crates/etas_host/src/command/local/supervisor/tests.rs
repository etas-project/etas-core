#[cfg(unix)]
use super::*;

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn missing_output_pipe_reaps_spawned_process_before_returning_error() {
    use std::{process::Stdio, time::Duration};

    for missing_stdout in [true, false] {
        let mut command = Command::new("/bin/sleep");
        command.arg("60").stdin(Stdio::null());
        command.stdout(if missing_stdout {
            Stdio::null()
        } else {
            Stdio::piped()
        });
        command.stderr(if missing_stdout {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        ProcessTreeController::configure(&mut command);
        let child = command.spawn().unwrap();
        let pid = libc::pid_t::try_from(child.id().unwrap()).unwrap();
        let (_cancel, cancellation) = oneshot::channel();
        let spawned = SpawnedCommand {
            child,
            operation: None,
            isolation: crate::CommandIsolationReport::trusted_unconfined(),
            input: None,
            cancellation,
            deadline: None,
            policy: CommandExecutionPolicy::default(),
            program: "/bin/sleep".into(),
        };
        let error = tokio::time::timeout(Duration::from_secs(3), spawned.run())
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(
            error.message,
            "spawned command is missing a configured output pipe"
        );
        let mut status = 0;
        // SAFETY: waitpid uses the test's child PID and a valid status pointer.
        let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        assert_eq!(waited, -1, "the supervisor must reap before returning");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}
