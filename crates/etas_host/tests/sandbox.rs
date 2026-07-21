use std::{fs, path::Path};

use etas_host::{
    CommandPolicy, DestructiveOpPolicy, FilesystemPolicy, HostErrorCode, NetworkEndpoint,
    NetworkPolicy, SandboxBroker, SandboxPolicy, TestWorkspace, WorkspaceDiffKind, WorkspaceRoot,
};

#[test]
fn workspace_paths_reject_traversal_and_absolute_escape() {
    let fixture = TestWorkspace::create("path-reject").expect("test workspace should create");
    let root = fixture.root().expect("workspace root should canonicalize");

    assert_eq!(
        root.resolve_for_create("../outside.txt")
            .expect_err("parent traversal should be rejected")
            .code,
        HostErrorCode::AuthorityDenied
    );
    assert_eq!(
        root.resolve_for_create("/tmp/outside.txt")
            .expect_err("absolute path should be rejected")
            .code,
        HostErrorCode::AuthorityDenied
    );

    fs::create_dir(fixture.path().join("dir")).expect("parent directory should exist");
    let normalized = root
        .resolve_for_create(Path::new("./dir//file.txt"))
        .expect("relative path with current-dir and repeated separators should normalize");
    assert_eq!(normalized.relative, Path::new("dir/file.txt"));
}

#[test]
fn workspace_path_normalization_covers_multiple_path_shapes() {
    let fixture = TestWorkspace::create("path-shapes").expect("test workspace should create");
    let root = fixture.root().expect("workspace root should canonicalize");
    fs::create_dir(fixture.path().join("safe")).expect("safe parent should exist");
    fs::create_dir(fixture.path().join("unicod\u{0435}"))
        .expect("unicode-looking parent should exist");

    let accepted = [
        (Path::new("safe/file.txt"), Path::new("safe/file.txt")),
        (Path::new("./safe//file.txt"), Path::new("safe/file.txt")),
        (
            Path::new("unicod\u{0435}/file.txt"),
            Path::new("unicod\u{0435}/file.txt"),
        ),
    ];
    for (input, expected) in accepted {
        let resolved = root
            .resolve_for_create(input)
            .expect("accepted path shape should normalize");
        assert_eq!(resolved.relative, expected);
    }

    for (rejected, code) in [
        (Path::new(""), HostErrorCode::InvalidRequest),
        (Path::new("."), HostErrorCode::InvalidRequest),
        (
            Path::new("safe/../escape.txt"),
            HostErrorCode::AuthorityDenied,
        ),
        (Path::new("../escape.txt"), HostErrorCode::AuthorityDenied),
    ] {
        assert_eq!(
            root.resolve_for_create(rejected)
                .expect_err("rejected path shape should fail")
                .code,
            code
        );
    }
}

#[test]
fn filesystem_denies_by_default_even_inside_workspace() {
    let fixture = TestWorkspace::create("deny-default").expect("test workspace should create");
    let root = fixture.root().expect("workspace root should canonicalize");
    let broker = SandboxBroker::new(SandboxPolicy::deny_all());

    assert_eq!(
        broker
            .atomic_write(&root, Path::new("file.txt"), b"blocked")
            .expect_err("deny-all sandbox should reject writes")
            .code,
        HostErrorCode::AuthorityDenied
    );
    assert!(!fixture.path().join("file.txt").exists());
}

#[test]
fn filesystem_atomic_write_read_and_delete_policy_are_explicit() {
    let fixture = TestWorkspace::create("filesystem").expect("test workspace should create");
    let root = fixture.root().expect("workspace root should canonicalize");
    let write_broker = SandboxBroker::new(SandboxPolicy::allow_listed(
        FilesystemPolicy::allow_workspace(root.clone()),
        NetworkPolicy::deny_all(),
        CommandPolicy::deny_all(),
        DestructiveOpPolicy::deny_all(),
    ));

    write_broker
        .atomic_write(&root, Path::new("file.txt"), b"hello")
        .expect("allowlisted workspace write should succeed");
    assert_eq!(
        write_broker
            .read_file(&root, Path::new("file.txt"))
            .expect("allowlisted workspace read should succeed"),
        b"hello"
    );
    assert_eq!(
        write_broker
            .delete_file(&root, Path::new("file.txt"))
            .expect_err("delete should require destructive policy")
            .code,
        HostErrorCode::AuthorityDenied
    );

    let delete_broker = SandboxBroker::new(SandboxPolicy::allow_listed(
        FilesystemPolicy::allow_destructive_workspace(root.clone()),
        NetworkPolicy::deny_all(),
        CommandPolicy::deny_all(),
        DestructiveOpPolicy::allow_workspace_delete(),
    ));
    delete_broker
        .delete_file(&root, Path::new("file.txt"))
        .expect("explicit destructive workspace policy should allow delete");
    assert!(!fixture.path().join("file.txt").exists());
}

#[cfg(unix)]
#[test]
fn filesystem_rejects_symlink_escape_for_reads_and_writes() {
    let fixture = TestWorkspace::create("symlink-root").expect("test workspace should create");
    let outside =
        TestWorkspace::create("symlink-outside").expect("outside workspace should create");
    let root = fixture.root().expect("workspace root should canonicalize");
    let broker = SandboxBroker::new(SandboxPolicy::allow_listed(
        FilesystemPolicy::allow_workspace(root.clone()),
        NetworkPolicy::deny_all(),
        CommandPolicy::deny_all(),
        DestructiveOpPolicy::deny_all(),
    ));

    fs::write(outside.path().join("secret.txt"), b"secret")
        .expect("outside file should be created");
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        fixture.path().join("leak.txt"),
    )
    .expect("test symlink should be created");
    assert_eq!(
        broker
            .read_file(&root, Path::new("leak.txt"))
            .expect_err("symlink to outside workspace should be rejected")
            .code,
        HostErrorCode::AuthorityDenied
    );

    fs::create_dir(outside.path().join("dir")).expect("outside directory should be created");
    std::os::unix::fs::symlink(
        outside.path().join("dir"),
        fixture.path().join("linked-dir"),
    )
    .expect("test directory symlink should be created");
    assert_eq!(
        broker
            .atomic_write(&root, Path::new("linked-dir/pwned.txt"), b"blocked")
            .expect_err("write through symlinked directory should be rejected")
            .code,
        HostErrorCode::AuthorityDenied
    );
    assert!(!outside.path().join("dir/pwned.txt").exists());
}

#[test]
fn network_and_command_sandboxes_deny_by_default_and_allow_exact_matches() {
    let root = TestWorkspace::create("policy").expect("test workspace should create");
    let workspace = root.root().expect("workspace root should canonicalize");
    let deny_broker = SandboxBroker::new(SandboxPolicy::allow_listed(
        FilesystemPolicy::allow_workspace(workspace.clone()),
        NetworkPolicy::deny_all(),
        CommandPolicy::deny_all(),
        DestructiveOpPolicy::deny_all(),
    ));

    assert_eq!(
        deny_broker
            .check_network_endpoint("http", "127.0.0.1", 8848)
            .expect_err("network should be denied without explicit endpoint")
            .code,
        HostErrorCode::AuthorityDenied
    );
    assert_eq!(
        deny_broker
            .check_command("python3")
            .expect_err("command should be denied without explicit program")
            .code,
        HostErrorCode::AuthorityDenied
    );

    let allow_broker = SandboxBroker::new(SandboxPolicy::allow_listed(
        FilesystemPolicy::allow_workspace(workspace),
        NetworkPolicy::allow_endpoints(vec![NetworkEndpoint::new("http", "127.0.0.1", 8848)]),
        CommandPolicy::allow_programs(vec!["python3".to_owned()]),
        DestructiveOpPolicy::deny_all(),
    ));
    allow_broker
        .check_network_endpoint("http", "127.0.0.1", 8848)
        .expect("exact local omlx endpoint should be allowlisted");
    assert_eq!(
        allow_broker
            .check_network_endpoint("http", "127.0.0.1", 8849)
            .expect_err("different port should not inherit allowlist")
            .code,
        HostErrorCode::AuthorityDenied
    );
    allow_broker
        .check_command("python3")
        .expect("exact command should be allowlisted");
}

#[test]
fn network_sandbox_rejects_alternative_ip_encodings_and_private_dns_results() {
    let alternative_hosts = [
        "0x7f000001",
        "2130706433",
        "017700000001",
        "0177.0.0.1",
        "0x7f.0.0.1",
    ];
    let sandbox = etas_host::NetworkSandbox::new(NetworkPolicy::allow_endpoints(
        alternative_hosts
            .iter()
            .map(|host| NetworkEndpoint::new("tcp", *host, 8848))
            .collect(),
    ));
    for host in alternative_hosts {
        let error = sandbox
            .check_endpoint("tcp", host, 8848)
            .expect_err("alternative IP encoding must fail closed");
        assert_eq!(error.code, HostErrorCode::AuthorityDenied);
        assert!(error.message.contains("non-canonical IP"), "{error:?}");
    }

    let localhost =
        etas_host::NetworkSandbox::new(NetworkPolicy::allow_endpoints(vec![NetworkEndpoint::new(
            "tcp",
            "localhost",
            8848,
        )]));
    let error = localhost
        .resolve_endpoint("tcp", "localhost", 8848)
        .expect_err("DNS names resolving to private addresses require an explicit IP grant");
    assert_eq!(error.code, HostErrorCode::AuthorityDenied);
    assert!(error.message.contains("resolved network address"));
    let loopback =
        etas_host::NetworkSandbox::new(NetworkPolicy::allow_endpoints(vec![NetworkEndpoint::new(
            "tcp",
            "127.0.0.1",
            8848,
        )]));
    let resolved = loopback
        .resolve_endpoint("tcp", "127.0.0.1", 8848)
        .expect("canonical explicitly allowed loopback address should resolve");
    assert_eq!(resolved.as_slice(), ["127.0.0.1:8848".parse().unwrap()]);
}

#[test]
fn snapshot_diff_and_rollback_restore_workspace_state() {
    let fixture = TestWorkspace::create("snapshot").expect("test workspace should create");
    let root = fixture.root().expect("workspace root should canonicalize");
    let broker = workspace_broker(root.clone());

    broker
        .atomic_write(&root, Path::new("existing.txt"), b"before")
        .expect("initial file should write");
    let snapshot = broker
        .snapshot(root.clone())
        .expect("snapshot should capture workspace");

    broker
        .atomic_write(&root, Path::new("existing.txt"), b"after")
        .expect("modified file should write");
    broker
        .atomic_write(&root, Path::new("added.txt"), b"new")
        .expect("added file should write");

    let diff = broker.diff(&snapshot).expect("diff should compute");
    assert_eq!(diff.entries.len(), 2);
    assert!(
        diff.entries
            .iter()
            .any(|entry| matches!(entry.kind, WorkspaceDiffKind::Added { .. }))
    );
    assert!(
        diff.entries
            .iter()
            .any(|entry| matches!(entry.kind, WorkspaceDiffKind::Modified { .. }))
    );

    let rollback_diff = broker.rollback(&snapshot).expect("rollback should restore");
    assert_eq!(rollback_diff.entries.len(), 2);
    assert_eq!(
        fs::read(fixture.path().join("existing.txt")).expect("existing file should remain"),
        b"before"
    );
    assert!(!fixture.path().join("added.txt").exists());
}

fn workspace_broker(root: WorkspaceRoot) -> SandboxBroker {
    SandboxBroker::new(SandboxPolicy::allow_listed(
        FilesystemPolicy::allow_workspace(root),
        NetworkPolicy::deny_all(),
        CommandPolicy::deny_all(),
        DestructiveOpPolicy::deny_all(),
    ))
}
