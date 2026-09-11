#![cfg(all(
    target_os = "linux",
    any(target_arch = "aarch64", target_arch = "x86_64")
))]

use etas_host::{
    AuthorityContext, CommandClient, CommandIsolation, CommandPolicy, CommandRequest,
    DestructiveOpPolicy, ExecutionBudget, FilesystemPolicy, HostActionGrant, HostErrorCode,
    HostRequestId, IsolationRequirements, LocalCommandClient, NetworkPolicy, PlatformSandboxHook,
    PolicyContext, SandboxPolicy, TestWorkspace, TraceContext, TraceId, WorkspaceRoot,
};
use std::{fs, path::Path, time::Duration};

#[path = "support/landlock_symlink_race.rs"]
mod symlink_race;

fn request(root: WorkspaceRoot, mode: &str) -> CommandRequest {
    let executable = std::env::current_exe().unwrap().display().to_string();
    #[cfg(target_arch = "aarch64")]
    let loader = "/lib/ld-linux-aarch64.so.1";
    #[cfg(target_arch = "x86_64")]
    let loader = "/lib64/ld-linux-x86-64.so.2";
    let mut read_roots = vec![root];
    // Explicit runtime dependencies of this test binary; no ambient / or /tmp grant.
    for path in ["/lib", "/lib64", "/usr/lib"] {
        if Path::new(path).is_dir() {
            read_roots.push(WorkspaceRoot::new(path).unwrap());
        }
    }
    CommandRequest {
        id: HostRequestId(1),
        argv: vec![
            executable.clone(),
            "--ignored".into(),
            "--exact".into(),
            "isolated_child_probe".into(),
            "--nocapture".into(),
        ],
        env: vec![
            ("ETAS_PROBE".into(), mode.into()),
            ("RUST_BACKTRACE".into(), "0".into()),
        ],
        cwd: None,
        stdin: None,
        authority: AuthorityContext {
            grants: vec![HostActionGrant::allow("Command", "run")],
            approvals: vec![],
            sandbox: SandboxPolicy::allow_listed(
                FilesystemPolicy {
                    read_roots,
                    write_roots: vec![],
                    delete_roots: vec![],
                },
                NetworkPolicy::deny_all(),
                CommandPolicy {
                    allowed_programs: vec![executable, loader.into()],
                    isolation: CommandIsolation::Required {
                        backend: PlatformSandboxHook::Landlock,
                        guarantees: IsolationRequirements::all(),
                    },
                },
                DestructiveOpPolicy::deny_all(),
            ),
            policy: PolicyContext::default(),
        },
        trace: TraceContext::root(TraceId(1)),
        budget: ExecutionBudget::default(),
    }
}

async fn execute(request: CommandRequest) -> etas_host::CommandOutput {
    let output = tokio::time::timeout(
        Duration::from_secs(10),
        LocalCommandClient::new().execute(request),
    )
    .await
    .expect("isolated child deadline")
    .expect("Landlock ABI v5 + seccomp must activate")
    .result
    .expect("child output");
    assert_eq!(
        output.exit_code,
        0,
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        output.isolation.backend(),
        Some(PlatformSandboxHook::Landlock)
    );
    assert_eq!(output.isolation.active(), IsolationRequirements::all());
    println!(
        "ETAS_ISOLATION_EVIDENCE backend=Landlock requested={:?} active={:?} cleanup=settled",
        IsolationRequirements::all(),
        output.isolation.active()
    );
    output
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a Linux kernel with Landlock ABI v5 enabled; run --ignored landlock_"]
async fn landlock_cancellation_reaps_the_isolated_child_before_scope_join() {
    use etas_host::execution::{CancellationReason, ExecutionScope, ExternalOutcome};
    let fixture = TestWorkspace::create("landlock-cancel").unwrap();
    let outside = TestWorkspace::create("landlock-cancel-outside").unwrap();
    fs::write(outside.path().join("secret"), "outside sentinel").unwrap();
    let ready = fixture.path().join("ready");
    let root = fixture.root().unwrap();
    let mut request = request(root.clone(), "cancel");
    request.authority.sandbox.filesystem.write_roots.push(root);
    request.env.extend([
        ("READY".into(), ready.display().to_string()),
        ("OUTSIDE".into(), outside.path().display().to_string()),
    ]);
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
    let ready_result = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(text) = fs::read_to_string(&ready)
                && let Ok(pid) = text.parse::<libc::pid_t>()
            {
                return Ok(pid);
            }
            if task.is_finished() {
                return Err("isolated child failed before confirming confinement");
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    // Always request cleanup before asserting readiness, including activation failure.
    scope
        .cancel_source()
        .stop(CancellationReason::Interrupt)
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .expect("isolated child cleanup deadline")
        .expect("supervisor task");
    registration
        .complete(ExternalOutcome::Unknown, vec![])
        .unwrap();
    scope.finish_body(true).unwrap();
    let report = tokio::time::timeout(Duration::from_secs(5), scope.join())
        .await
        .expect("scope cleanup deadline")
        .unwrap();
    assert!(
        report
            .operations()
            .iter()
            .all(|operation| operation.cleanup_errors().is_empty()),
        "{report:?}"
    );
    let pid = ready_result
        .expect("isolated child readiness deadline")
        .unwrap_or_else(|message| panic!("{message}: {result:?}"));
    assert_eq!(result.unwrap_err().code, HostErrorCode::Cancelled);
    assert_eq!(
        unsafe { libc::kill(pid, 0) },
        -1,
        "child must have been reaped"
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
    println!("ETAS_ISOLATION_CLEANUP cancelled=true reaped=true cleanup=settled");
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a Linux kernel with Landlock ABI v5 enabled; run --ignored landlock_"]
async fn landlock_enforces_read_write_delete_and_outside_confinement() {
    let fixture = TestWorkspace::create("landlock-granted").unwrap();
    let outside = TestWorkspace::create("landlock-outside").unwrap();
    fs::write(fixture.path().join("readable"), "inside").unwrap();
    fs::copy(
        std::env::current_exe().unwrap(),
        fixture.path().join("unlisted-executable"),
    )
    .unwrap();
    fs::write(outside.path().join("secret"), "outside").unwrap();
    std::os::unix::fs::symlink(outside.path(), fixture.path().join("escape")).unwrap();
    let root = fixture.root().unwrap();
    let mut read_only = request(root.clone(), "filesystem");
    read_only.env.extend([
        ("ROOT".into(), fixture.path().display().to_string()),
        ("OUTSIDE".into(), outside.path().display().to_string()),
    ]);
    execute(read_only.clone()).await;
    read_only.env.push(("WRITE_ALLOWED".into(), "yes".into()));
    read_only
        .authority
        .sandbox
        .filesystem
        .write_roots
        .push(root.clone());
    execute(read_only.clone()).await;
    read_only.env.push(("DELETE_ALLOWED".into(), "yes".into()));
    read_only
        .authority
        .sandbox
        .filesystem
        .delete_roots
        .push(root);
    read_only.authority.sandbox.destructive_ops = DestructiveOpPolicy::allow_workspace_delete();
    execute(read_only).await;
    assert_eq!(
        fs::read_to_string(outside.path().join("secret")).unwrap(),
        "outside"
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a Linux kernel with Landlock ABI v5 enabled; run --ignored landlock_"]
async fn landlock_retains_directory_object_after_root_replacement() {
    let fixture = TestWorkspace::create("landlock-root-race").unwrap();
    let original = fixture.path().join("root");
    let retained = fixture.path().join("retained");
    fs::create_dir(&original).unwrap();
    fs::write(original.join("readable"), "inside").unwrap();
    let root = WorkspaceRoot::new(&original).unwrap();
    fs::rename(&original, &retained).unwrap();
    fs::create_dir(&original).unwrap();
    fs::write(original.join("secret"), "replacement").unwrap();
    let mut request = request(root, "retained");
    request.env.extend([
        ("ROOT".into(), retained.display().to_string()),
        ("OUTSIDE".into(), original.display().to_string()),
    ]);
    execute(request).await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a Linux kernel with Landlock ABI v5 enabled; run --ignored landlock_"]
async fn landlock_enforces_confinement_during_symlink_replacement() {
    symlink_race::run().await;
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a Linux kernel with Landlock ABI v5 enabled; run --ignored landlock_"]
async fn landlock_blocks_network_peer_process_access_and_inherited_descriptors() {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    let fixture = TestWorkspace::create("landlock-protocols").unwrap();
    let secret = fs::File::open(fixture.path()).unwrap();
    let fd = unsafe { libc::fcntl(secret.as_raw_fd(), libc::F_DUPFD, 200) };
    assert!(fd >= 200);
    let descriptor = unsafe { OwnedFd::from_raw_fd(fd) };
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut request = request(fixture.root().unwrap(), "protocols");
    request.env.extend([
        ("PARENT_PID".into(), std::process::id().to_string()),
        ("SECRET_FD".into(), descriptor.as_raw_fd().to_string()),
        (
            "ENDPOINT".into(),
            listener.local_addr().unwrap().to_string(),
        ),
    ]);
    execute(request).await;
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "requires a Linux kernel with Landlock ABI v5 enabled; run --ignored landlock_"]
async fn landlock_exec_failure_never_runs_unconfined() {
    let fixture = TestWorkspace::create("landlock-denied-loader").unwrap();
    let mut denied = request(fixture.root().unwrap(), "must-not-execute");
    // Only authorize the binary, not its ELF interpreter. The child must fail
    // at exec, not retry without the installed policy or report activation.
    denied
        .authority
        .sandbox
        .command
        .allowed_programs
        .truncate(1);
    let error = LocalCommandClient::new().execute(denied).await.unwrap_err();
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
    assert!(error.message.contains("spawn"));
    assert!(
        error
            .details
            .iter()
            .any(|detail| { detail.key == "error" && detail.value.contains("Permission denied") })
    );
}

#[tokio::test(flavor = "current_thread")]
async fn landlock_does_not_guess_endpoint_rules_or_retry_invalid_exec_unconfined() {
    let fixture = TestWorkspace::create("landlock-invalid-policy").unwrap();
    let mut network = request(fixture.root().unwrap(), "must-not-execute");
    network.authority.sandbox.network =
        NetworkPolicy::allow_endpoints(vec![etas_host::NetworkEndpoint::new(
            "tcp",
            "127.0.0.1",
            8080,
        )]);
    assert_eq!(
        LocalCommandClient::new()
            .execute(network)
            .await
            .unwrap_err()
            .code,
        HostErrorCode::InvalidRequest
    );
    let mut invalid = request(fixture.root().unwrap(), "must-not-execute");
    invalid
        .authority
        .sandbox
        .command
        .allowed_programs
        .push("not-an-absolute-program".into());
    assert_eq!(
        LocalCommandClient::new()
            .execute(invalid)
            .await
            .unwrap_err()
            .code,
        HostErrorCode::InvalidRequest
    );
}

#[test]
#[ignore = "executed only as the real isolated subprocess by the parent tests"]
fn isolated_child_probe() {
    let mode = std::env::var("ETAS_PROBE").expect("parent must supply probe mode");
    match mode.as_str() {
        "symlink-race" => symlink_race::probe(),
        "cancel" => {
            let outside = std::path::PathBuf::from(std::env::var("OUTSIDE").unwrap());
            assert_eq!(
                fs::read(outside.join("secret")).unwrap_err().raw_os_error(),
                Some(libc::EACCES)
            );
            fs::write(
                std::env::var("READY").unwrap(),
                std::process::id().to_string(),
            )
            .unwrap();
            loop {
                std::thread::park();
            }
        }
        "filesystem" | "retained" => {
            let root = std::path::PathBuf::from(std::env::var("ROOT").unwrap());
            let outside = std::path::PathBuf::from(std::env::var("OUTSIDE").unwrap());
            assert_eq!(fs::read_to_string(root.join("readable")).unwrap(), "inside");
            assert_eq!(
                fs::read(outside.join("secret")).unwrap_err().raw_os_error(),
                Some(libc::EACCES)
            );
            assert!(fs::write(outside.join("created"), "must not write").is_err());
            assert!(fs::read(root.join("escape/secret")).is_err());
            let traversal = root
                .join("..")
                .join(outside.file_name().unwrap())
                .join("secret");
            assert!(fs::read(traversal).is_err());
            if mode == "filesystem" {
                assert_eq!(
                    std::process::Command::new(root.join("unlisted-executable"))
                        .status()
                        .unwrap_err()
                        .raw_os_error(),
                    Some(libc::EACCES)
                );
                let result = fs::write(root.join("written"), "new");
                if std::env::var_os("WRITE_ALLOWED").is_some() {
                    result.unwrap();
                    let removed = fs::remove_file(root.join("written"));
                    if std::env::var_os("DELETE_ALLOWED").is_some() {
                        removed.unwrap();
                    } else {
                        assert!(removed.is_err());
                    }
                } else {
                    assert!(result.is_err());
                }
                let child = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--ignored",
                        "--exact",
                        "isolated_child_probe",
                        "--nocapture",
                    ])
                    .env("ETAS_PROBE", "retained")
                    .stdin(std::process::Stdio::inherit())
                    .output()
                    .unwrap();
                assert!(child.status.success(), "{child:?}");
            }
        }
        "protocols" => {
            assert_eq!(
                std::net::TcpStream::connect(std::env::var("ENDPOINT").unwrap())
                    .unwrap_err()
                    .raw_os_error(),
                Some(libc::EPERM)
            );
            assert_eq!(
                std::net::UdpSocket::bind("127.0.0.1:0")
                    .unwrap_err()
                    .raw_os_error(),
                Some(libc::EPERM)
            );
            assert_eq!(
                std::os::unix::net::UnixStream::pair()
                    .unwrap_err()
                    .raw_os_error(),
                Some(libc::EPERM)
            );
            let pid = std::env::var("PARENT_PID").unwrap().parse::<i32>().unwrap();
            assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EPERM)
            );
            assert_eq!(unsafe { libc::setsid() }, -1);
            assert_eq!(unsafe { libc::unshare(0) }, -1);
            let fd = std::env::var("SECRET_FD").unwrap().parse::<i32>().unwrap();
            assert_eq!(unsafe { libc::fcntl(fd, libc::F_GETFD) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::EBADF)
            );
            // A real libc thread clone must still work under the constrained ABI.
            assert_eq!(std::thread::spawn(|| 42).join().unwrap(), 42);
        }
        _ => panic!("unexpected or unconfined probe execution"),
    }
}
