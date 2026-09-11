use std::time::{Duration, Instant};

#[cfg(unix)]
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use etas_host::{
    AuthorityContext, Budget, CommandClient, CommandExecutionPolicy, CommandOutput, CommandPolicy,
    CommandRequest, DestructiveOpPolicy, ExecutionBudget, FilesystemPolicy, HostActionGrant,
    HostErrorCode, HostRequestId, LocalCommandClient, NetworkPolicy, PolicyContext, SandboxPolicy,
    TimeBudget, TraceContext, TraceId,
};

#[tokio::test]
async fn local_command_rejects_missing_command_grant() {
    let program = "/bin/echo";
    let client = LocalCommandClient::new();
    let error = client
        .execute(command_request(
            program,
            Vec::new(),
            SandboxPolicy::allow_listed(
                FilesystemPolicy::deny_all(),
                NetworkPolicy::deny_all(),
                CommandPolicy::allow_trusted_programs(vec![program.to_owned()]),
                DestructiveOpPolicy::deny_all(),
            ),
        ))
        .await
        .expect_err("command must fail closed without a checked grant");

    assert_eq!(error.code, HostErrorCode::AuthorityDenied);
}

#[tokio::test]
async fn local_command_executes_allowlisted_program_with_grant() {
    let program = "/bin/echo";
    let client = LocalCommandClient::new();
    let response = client
        .execute(command_request(
            program,
            vec![HostActionGrant::allow("Command", "run")],
            SandboxPolicy::allow_listed(
                FilesystemPolicy::deny_all(),
                NetworkPolicy::deny_all(),
                CommandPolicy::allow_trusted_programs(vec![program.to_owned()]),
                DestructiveOpPolicy::deny_all(),
            ),
        ))
        .await
        .expect("allowlisted command should execute");
    let CommandOutput {
        isolation,
        exit_code,
        stdout,
        stderr,
    } = response.result.expect("command should return output");

    assert_eq!(exit_code, 0);
    assert_eq!(
        isolation.active(),
        etas_host::IsolationRequirements::default()
    );
    assert_eq!(isolation.backend(), None);
    assert_eq!(stdout, b"etas\n");
    assert!(stderr.is_empty());
}

#[tokio::test]
async fn local_command_enforces_run_owned_deadline_and_reaps_process() {
    let program = "/bin/sh";
    let client = LocalCommandClient::with_policy(CommandExecutionPolicy::default());
    let request = authorized_command_request(
        vec![
            program.to_owned(),
            "-c".to_owned(),
            "/bin/sleep 30".to_owned(),
        ],
        ExecutionBudget::start(Budget {
            time: Some(TimeBudget { max_millis: 50 }),
            ..Budget::default()
        }),
    );

    let started = Instant::now();
    let error = client
        .execute(request)
        .await
        .expect_err("the run-owned deadline must terminate the command");

    assert_eq!(error.code, HostErrorCode::BudgetExceeded, "{error}");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "deadline enforcement took {:?}",
        started.elapsed()
    );
}

#[tokio::test]
async fn local_command_rejects_stdout_overflow_with_typed_error() {
    let program = "/bin/sh";
    let client = LocalCommandClient::with_policy(CommandExecutionPolicy::new(8, 64));
    let request = authorized_command_request(
        vec![
            program.to_owned(),
            "-c".to_owned(),
            "printf 123456789; /bin/sleep 30".to_owned(),
        ],
        ExecutionBudget::default(),
    );

    let error = client
        .execute(request)
        .await
        .expect_err("stdout above the configured bound must fail");

    assert_eq!(error.code, HostErrorCode::BudgetExceeded, "{error}");
    assert!(
        error
            .details
            .iter()
            .any(|detail| detail.key == "stream" && detail.value == "stdout")
    );
    assert!(
        error
            .details
            .iter()
            .any(|detail| detail.key == "limit_bytes" && detail.value == "8")
    );
}

#[tokio::test]
async fn local_command_rejects_stderr_overflow_with_typed_error() {
    let program = "/bin/sh";
    let client = LocalCommandClient::with_policy(CommandExecutionPolicy::new(64, 8));
    let request = authorized_command_request(
        vec![
            program.to_owned(),
            "-c".to_owned(),
            "printf 123456789 >&2; /bin/sleep 30".to_owned(),
        ],
        ExecutionBudget::default(),
    );

    let error = client
        .execute(request)
        .await
        .expect_err("stderr above the configured bound must fail");

    assert_eq!(error.code, HostErrorCode::BudgetExceeded, "{error}");
    assert!(
        error
            .details
            .iter()
            .any(|detail| detail.key == "stream" && detail.value == "stderr")
    );
    assert!(
        error
            .details
            .iter()
            .any(|detail| detail.key == "limit_bytes" && detail.value == "8")
    );
}

#[tokio::test]
async fn local_command_drains_stdout_and_stderr_concurrently() {
    let program = "/bin/sh";
    let client = LocalCommandClient::with_policy(CommandExecutionPolicy::new(8192, 8192));
    let request = authorized_command_request(
        vec![
            program.to_owned(),
            "-c".to_owned(),
            "i=0; while [ $i -lt 4096 ]; do printf o; printf e >&2; i=$((i+1)); done".to_owned(),
        ],
        ExecutionBudget::default(),
    );

    let output = client
        .execute(request)
        .await
        .expect("bounded command should execute")
        .result
        .expect("bounded command should return output");

    assert_eq!(output.stdout, vec![b'o'; 4096]);
    assert_eq!(output.stderr, vec![b'e'; 4096]);
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_command_future_terminates_and_reaps_process_tree() {
    let program = "/bin/sh";
    let pid_file = unique_pid_file();
    let request = authorized_command_request(
        vec![
            program.to_owned(),
            "-c".to_owned(),
            "/bin/sleep 30 & child=$!; printf '%s %s\n' \"$$\" \"$child\" > \"$1\"; wait \"$child\""
                .to_owned(),
            "fixture".to_owned(),
            pid_file.display().to_string(),
        ],
        ExecutionBudget::default(),
    );
    let client = LocalCommandClient::new();
    let task = tokio::spawn(async move { client.execute(request).await });
    let (parent, descendant) = wait_for_pids(&pid_file).await;
    let mut cleanup = ProcessCleanup(vec![parent, descendant]);

    task.abort();
    assert!(
        task.await
            .expect_err("task must be cancelled")
            .is_cancelled()
    );
    wait_until_process_exits(parent).await;
    wait_until_process_exits(descendant).await;

    assert!(
        !process_is_live(parent),
        "command process {parent} survived"
    );
    assert!(
        !process_is_live(descendant),
        "command descendant {descendant} survived"
    );
    std::fs::remove_file(&pid_file).ok();
    cleanup.0.clear();
}

#[cfg(unix)]
#[tokio::test]
async fn command_deadline_kills_descendants_after_parent_has_exited() {
    let pid_file = unique_pid_file();
    let request = authorized_command_request(
        vec![
            "/bin/sh".into(),
            "-c".into(),
            "/bin/sleep 30 & child=$!; printf '%s %s\\n' \"$$\" \"$child\" > \"$1\"; exit 0".into(),
            "fixture".into(),
            pid_file.display().to_string(),
        ],
        ExecutionBudget::start(Budget {
            time: Some(TimeBudget { max_millis: 500 }),
            ..Budget::default()
        }),
    );
    let task = tokio::spawn(async move { LocalCommandClient::new().execute(request).await });
    let (parent, descendant) = wait_for_pids(&pid_file).await;
    let _cleanup = ProcessCleanup(vec![descendant]);
    wait_until_process_exits(parent).await;
    assert!(
        !process_is_live(parent),
        "parent must exit before the deadline"
    );
    let error = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .expect("deadline must finish command supervision")
        .expect("supervisor task")
        .expect_err("descendant keeps stdout open beyond the deadline");
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    wait_until_process_exits(descendant).await;
    std::fs::remove_file(pid_file).ok();
    assert!(
        !process_is_live(descendant),
        "descendant survived after its parent exited"
    );
}

fn command_request(
    program: &str,
    grants: Vec<HostActionGrant>,
    sandbox: SandboxPolicy,
) -> CommandRequest {
    CommandRequest {
        id: HostRequestId(1),
        argv: vec![program.to_owned(), "etas".to_owned()],
        env: Vec::new(),
        cwd: None,
        stdin: None,
        authority: AuthorityContext {
            grants,
            approvals: Vec::new(),
            sandbox,
            policy: PolicyContext::default(),
        },
        trace: TraceContext::root(TraceId(1)),
        budget: etas_host::ExecutionBudget::default(),
    }
}

fn authorized_command_request(argv: Vec<String>, budget: ExecutionBudget) -> CommandRequest {
    let program = argv.first().expect("test command has a program").clone();
    CommandRequest {
        id: HostRequestId(2),
        argv,
        env: Vec::new(),
        cwd: None,
        stdin: None,
        authority: AuthorityContext {
            grants: vec![HostActionGrant::allow("Command", "run")],
            approvals: Vec::new(),
            sandbox: SandboxPolicy::allow_listed(
                FilesystemPolicy::deny_all(),
                NetworkPolicy::deny_all(),
                CommandPolicy::allow_trusted_programs(vec![program]),
                DestructiveOpPolicy::deny_all(),
            ),
            policy: PolicyContext::default(),
        },
        trace: TraceContext::root(TraceId(2)),
        budget,
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn required_isolation_rejects_before_spawn_for_every_unavailable_backend() {
    use etas_host::{CommandIsolation, IsolationRequirements, PlatformSandboxHook};
    let marker = unique_pid_file();
    for backend in [
        #[cfg(not(target_os = "linux"))]
        PlatformSandboxHook::Landlock,
        PlatformSandboxHook::Container,
        PlatformSandboxHook::WasiPreopen,
    ] {
        let mut request = authorized_command_request(
            vec![
                "/bin/sh".into(),
                "-c".into(),
                "printf executed > \"$1\"".into(),
                "fixture".into(),
                marker.display().to_string(),
            ],
            ExecutionBudget::default(),
        );
        request.authority.sandbox.command.isolation = CommandIsolation::Required {
            backend,
            guarantees: IsolationRequirements::all(),
        };
        let error = LocalCommandClient::new()
            .execute(request)
            .await
            .unwrap_err();
        assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
        assert!(
            !marker.exists(),
            "activation failure must not execute the child"
        );
    }
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn child_inherits_stdio_but_not_parent_secret_or_directory_descriptors() {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let directory = unique_pid_file();
    std::fs::create_dir(&directory).unwrap();
    let secret = directory.join("secret");
    std::fs::write(&secret, "parent-only-secret").unwrap();
    let file = std::fs::File::open(&secret).unwrap();
    let root = std::fs::File::open(&directory).unwrap();
    // Simulate a foreign library leaking inheritable handles. High descriptors
    // avoid collisions with shell-owned descriptors after exec.
    let inheritable: Vec<OwnedFd> = [&file, &root]
        .into_iter()
        .map(|file| unsafe {
            let fd = libc::fcntl(file.as_raw_fd(), libc::F_DUPFD, 200);
            assert!(fd >= 200);
            OwnedFd::from_raw_fd(fd)
        })
        .collect();
    let request = authorized_command_request(
        vec![
            "/bin/sh".into(),
            "-c".into(),
            format!(
                "if /bin/cat /dev/fd/{} 2>/dev/null; then exit 90; fi; if /bin/cat /dev/fd/{}/secret 2>/dev/null; then exit 91; fi; printf stdio-ok",
                inheritable[0].as_raw_fd(),
                inheritable[1].as_raw_fd()
            ),
        ],
        ExecutionBudget::default(),
    );
    let parent_cwd = std::env::current_dir().unwrap();
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        LocalCommandClient::new().execute(request),
    )
    .await
    .unwrap()
    .unwrap()
    .result
    .unwrap();
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, b"stdio-ok");
    assert_eq!(std::env::current_dir().unwrap(), parent_cwd);
    for descriptor in &inheritable {
        assert_eq!(
            unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0,
            "descriptor flags must only change in the child"
        );
    }
    assert_eq!(
        std::fs::read_to_string(secret).unwrap(),
        "parent-only-secret"
    );
    drop(inheritable);
    std::fs::remove_dir_all(directory).unwrap();
}

#[cfg(unix)]
fn unique_pid_file() -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "etas-command-{}-{}.pid",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn execution_scope_stop_waits_for_command_supervisor_reap() {
    use etas_host::execution::{CancellationReason, ExecutionScope, ExternalOutcome};
    let file = unique_pid_file();
    let request = authorized_command_request(vec![
        "/bin/sh".into(), "-c".into(),
        "/bin/sleep 30 & child=$!; printf '%s %s\n' \"$$\" \"$child\" > \"$1\"; wait \"$child\"".into(),
        "fixture".into(), file.display().to_string(),
    ], ExecutionBudget::default());
    let scope = ExecutionScope::new();
    let registration = scope
        .register(None, Some(request.id), request.trace.clone())
        .unwrap();
    registration.begin_dispatch().unwrap();
    let context = registration.context().clone();
    let task = tokio::spawn(async move {
        LocalCommandClient::new()
            .execute_scoped(request, &context)
            .await
    });
    let (parent, descendant) = wait_for_pids(&file).await;
    let mut cleanup = ProcessCleanup(vec![parent, descendant]);
    scope
        .cancel_source()
        .stop(CancellationReason::Interrupt)
        .unwrap();
    let error = tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, HostErrorCode::Cancelled);
    registration
        .complete(ExternalOutcome::Unknown, vec![])
        .unwrap();
    scope.finish_body(true).unwrap();
    scope.join().await.unwrap();
    assert!(!process_is_live(parent), "join must wait for reap");
    wait_until_process_exits(descendant).await;
    assert!(!process_is_live(descendant));
    cleanup.0.clear();
    std::fs::remove_file(file).unwrap();
}

#[cfg(unix)]
async fn wait_for_pids(path: &Path) -> (libc::pid_t, libc::pid_t) {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Ok(value) = std::fs::read_to_string(path) {
            let mut pids = value.split_whitespace();
            if let (Some(parent), Some(descendant)) = (pids.next(), pids.next()) {
                return (
                    parent.parse().expect("parent pid"),
                    descendant.parse().expect("descendant pid"),
                );
            }
        }
        assert!(
            Instant::now() < deadline,
            "command did not publish process identities"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(unix)]
async fn wait_until_process_exits(process_id: libc::pid_t) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while process_is_live(process_id) && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[cfg(unix)]
fn process_is_live(process_id: libc::pid_t) -> bool {
    // SAFETY: signal 0 only queries the kernel for the supplied process identity.
    let result = unsafe { libc::kill(process_id, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(unix)]
struct ProcessCleanup(Vec<libc::pid_t>);

#[cfg(unix)]
impl Drop for ProcessCleanup {
    fn drop(&mut self) {
        for process_id in &self.0 {
            // SAFETY: this is best-effort cleanup of process IDs created by this test.
            unsafe {
                libc::kill(*process_id, libc::SIGKILL);
            }
        }
    }
}
