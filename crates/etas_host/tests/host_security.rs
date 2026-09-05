use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use etas_host::console::{ConsoleClient, ConsoleOperation, ConsoleRequest, LocalStdioClient};
use etas_host::{
    AnthropicProtocolAdapter, AuthorityContext, BrowserProtocolClient, BrowserProtocolOperation,
    BrowserProtocolRequest, FilesystemClient, FilesystemEntry, FilesystemOperation,
    FilesystemRequest, HostActionGrant, HostErrorCode, HostRequestId, HttpToolProtocolAdapter,
    HttpTransport, InMemoryMemoryClient, LocalFilesystemClient, MemoryClient, MemoryOperation,
    MemoryRequest, MemoryResult, MemoryVersion, ModelClient, ModelContent, ModelMessage, ModelName,
    ModelOptions, ModelRequest, ModelRole, NetworkPolicy, OpenAiProtocolAdapter,
    ProcessToolProtocolAdapter, RetryPolicy, SandboxPolicy, SecretClient, SecretOperation,
    SecretRequest, SecretValue, StoreRef, StreamClient, StreamOperation, StreamRequest, TcpClient,
    TcpConnectOperation, TcpConnectRequest, TcpEndpoint, TestWorkspace, ToolClient, ToolRef,
    ToolRequest, TraceContext, TraceId, TransportTimeoutPolicy, UnavailableBrowserProtocolClient,
    UnavailableSecretClient, UnavailableStreamClient, UnavailableTcpClient, WorkspacePath,
};

#[tokio::test]
async fn model_adapter_transport_does_not_use_program_sandbox() {
    let openai_error = OpenAiProtocolAdapter::omlx_compatible("http://127.0.0.1:9/v1")
        .expect("local endpoint syntax should be valid")
        .complete(model_request(HostRequestId(1), SandboxPolicy::deny_all()))
        .await
        .expect_err("test endpoint should not provide an OpenAI response");
    assert_ne!(openai_error.code, HostErrorCode::AuthorityDenied);

    let anthropic_error = AnthropicProtocolAdapter::omlx_compatible("http://127.0.0.1:9")
        .expect("local endpoint syntax should be valid")
        .complete(model_request(HostRequestId(2), SandboxPolicy::deny_all()))
        .await
        .expect_err("test endpoint should not provide an Anthropic response");
    assert_ne!(anthropic_error.code, HostErrorCode::AuthorityDenied);
}

#[tokio::test]
async fn http_tool_adapter_transport_does_not_use_program_sandbox() {
    let adapter = HttpToolProtocolAdapter::try_new_with_policy(
        "http://127.0.0.1:9",
        "/tool",
        etas_host::PrivateResolutionPolicy::AllowPrivate,
    )
    .expect("local tool endpoint syntax should be valid");
    let error = adapter
        .invoke(tool_request(HostRequestId(3), SandboxPolicy::deny_all()))
        .await
        .expect_err("test endpoint should not provide a tool response");
    assert_ne!(error.code, HostErrorCode::AuthorityDenied);
}

#[tokio::test]
async fn process_tool_rejects_command_when_sandbox_denies_all() {
    let adapter =
        ProcessToolProtocolAdapter::new("definitely-not-a-real-etas-test-command", Vec::new());
    let error = adapter
        .invoke(tool_request(HostRequestId(4), SandboxPolicy::deny_all()))
        .await
        .expect_err("process tool should reject before spawning");
    assert_eq!(error.code, HostErrorCode::AuthorityDenied);
}

#[tokio::test]
async fn local_stdio_client_rejects_console_without_action_grant() {
    let adapter = LocalStdioClient::new();
    let error = adapter
        .execute(ConsoleRequest {
            id: HostRequestId(5),
            operation: ConsoleOperation::WriteStdout {
                text: "should not write".to_owned(),
                newline: true,
            },
            authority: AuthorityContext {
                grants: Vec::new(),
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(5)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect_err("console client should reject before local IO");
    assert_eq!(error.code, HostErrorCode::AuthorityDenied);
}

#[tokio::test]
async fn local_stdio_client_rejects_mismatched_console_action_grant() {
    let adapter = LocalStdioClient::new();
    let error = adapter
        .execute(ConsoleRequest {
            id: HostRequestId(6),
            operation: ConsoleOperation::WriteStdout {
                text: "should not write".to_owned(),
                newline: true,
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Console", "stdin_read_line")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(6)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect_err("console client should reject mismatched action grant");
    assert_eq!(error.code, HostErrorCode::AuthorityDenied);
    assert!(
        error
            .details
            .iter()
            .any(|detail| { detail.key == "action" && detail.value == "Console.stdout_write" })
    );
}

#[tokio::test]
async fn local_filesystem_client_rejects_path_escape() {
    let workspace = TestWorkspace::create("fs-escape").expect("workspace");
    let root = workspace.root().expect("root");
    let adapter = LocalFilesystemClient::new();
    let response = adapter
        .execute(FilesystemRequest {
            id: HostRequestId(7),
            operation: FilesystemOperation::Read {
                path: WorkspacePath {
                    root: root.clone(),
                    relative: PathBuf::from("../outside"),
                },
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Fs", "read")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::allow_listed(
                    etas_host::FilesystemPolicy {
                        read_roots: vec![root],
                        write_roots: Vec::new(),
                        delete_roots: Vec::new(),
                    },
                    NetworkPolicy::deny_all(),
                    etas_host::CommandPolicy::allow_programs(Vec::new()),
                    etas_host::DestructiveOpPolicy::deny_all(),
                ),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(7)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("client should return response");
    assert_eq!(
        response
            .result
            .expect_err("path escape should be rejected")
            .code,
        HostErrorCode::AuthorityDenied
    );
}

#[tokio::test]
async fn local_filesystem_client_writes_root_file_with_create_dirs() {
    let workspace = TestWorkspace::create("fs-root-write").expect("workspace");
    let root = workspace.root().expect("root");
    let adapter = LocalFilesystemClient::new();
    let response = adapter
        .execute(FilesystemRequest {
            id: HostRequestId(8),
            operation: FilesystemOperation::Write {
                path: WorkspacePath {
                    root: root.clone(),
                    relative: PathBuf::from("out.txt"),
                },
                contents: b"ok".to_vec(),
                create_dirs: true,
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Fs", "write")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::allow_listed(
                    etas_host::FilesystemPolicy {
                        read_roots: Vec::new(),
                        write_roots: vec![root.clone()],
                        delete_roots: Vec::new(),
                    },
                    NetworkPolicy::deny_all(),
                    etas_host::CommandPolicy::allow_programs(Vec::new()),
                    etas_host::DestructiveOpPolicy::deny_all(),
                ),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(8)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("client should return response");
    assert_eq!(
        response.result.expect("root write should succeed"),
        FilesystemEntry::Unit
    );
    assert_eq!(
        std::fs::read_to_string(root.canonical_root.join("out.txt")).expect("written file"),
        "ok"
    );
}

#[tokio::test]
async fn local_filesystem_client_stats_and_atomic_replaces_under_workspace_policy() {
    let workspace = TestWorkspace::create("fs-stat-replace").expect("workspace");
    let root = workspace.root().expect("root");
    let adapter = LocalFilesystemClient::new();
    std::fs::write(root.canonical_root.join("data.bin"), b"old").expect("fixture file");

    let stat = adapter
        .execute(FilesystemRequest {
            id: HostRequestId(9),
            operation: FilesystemOperation::Stat {
                path: WorkspacePath {
                    root: root.clone(),
                    relative: PathBuf::from("data.bin"),
                },
            },
            authority: fs_authority(root.clone()),
            trace: TraceContext::root(TraceId(9)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("filesystem response");
    let FilesystemEntry::Stat(stat) = stat.result.expect("stat should succeed") else {
        panic!("expected filesystem stat response");
    };
    assert!(stat.is_file);
    assert!(!stat.is_dir);
    assert_eq!(stat.len, 3);

    let replace = adapter
        .execute(FilesystemRequest {
            id: HostRequestId(10),
            operation: FilesystemOperation::AtomicReplace {
                path: WorkspacePath {
                    root: root.clone(),
                    relative: PathBuf::from("data.bin"),
                },
                contents: b"new".to_vec(),
            },
            authority: fs_authority(root.clone()),
            trace: TraceContext::root(TraceId(10)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("filesystem response");
    assert_eq!(
        replace.result.expect("atomic replace should succeed"),
        FilesystemEntry::Unit
    );
    assert_eq!(
        std::fs::read(root.canonical_root.join("data.bin")).expect("replaced file"),
        b"new"
    );
}

#[tokio::test]
async fn tcp_client_rejects_unallowlisted_endpoint_before_provider_lookup() {
    let adapter = UnavailableTcpClient;
    let response = adapter
        .execute(TcpConnectRequest {
            id: HostRequestId(11),
            operation: TcpConnectOperation::Connect {
                endpoint: TcpEndpoint {
                    host: "127.0.0.1".to_owned(),
                    port: 6553,
                },
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Net", "tcp_connect")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::allow_listed(
                    etas_host::FilesystemPolicy::deny_all(),
                    NetworkPolicy::deny_all(),
                    etas_host::CommandPolicy::allow_programs(Vec::new()),
                    etas_host::DestructiveOpPolicy::deny_all(),
                ),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(11)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("tcp response");
    assert_eq!(
        response
            .result
            .expect_err("unallowlisted TCP endpoint should be rejected")
            .code,
        HostErrorCode::AuthorityDenied
    );
}

#[tokio::test]
async fn stream_read_is_bounded_before_unavailable_provider_error() {
    let adapter = UnavailableStreamClient::new(4);
    let response = adapter
        .execute(StreamRequest {
            id: HostRequestId(12),
            operation: StreamOperation::Read {
                stream: etas_host::ByteStreamRef::opaque_for_testing("s1", 0),
                max_bytes: 8,
                timeout_ms: None,
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Stream", "read")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(12)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("stream response");
    assert_eq!(
        response
            .result
            .expect_err("over-limit stream read should be rejected"),
        etas_host::StreamFailure::LimitExceeded { limit_bytes: 4 }
    );
}

#[tokio::test]
async fn stream_read_until_limit_is_bounded_before_unavailable_provider_error() {
    let adapter = UnavailableStreamClient::new(4);
    let response = adapter
        .execute(StreamRequest {
            id: HostRequestId(13),
            operation: StreamOperation::ReadUntilLimit {
                stream: etas_host::ByteStreamRef::opaque_for_testing("s1", 0),
                limit_bytes: 8,
                timeout_ms: None,
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Stream", "read")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(13)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("stream response");
    assert_eq!(
        response
            .result
            .expect_err("over-limit stream read-until-limit should be rejected"),
        etas_host::StreamFailure::LimitExceeded { limit_bytes: 4 }
    );
}

#[test]
fn secret_value_does_not_expose_raw_material() {
    let value = SecretValue::new(
        etas_host::SecretRef::new("env:ETAS_TEST_SECRET"),
        "ETAS_TEST_SECRET=<redacted>",
    );
    assert_eq!(value.reference().id(), "env:ETAS_TEST_SECRET");
    assert_eq!(value.redacted_label(), "ETAS_TEST_SECRET=<redacted>");
    assert!(!format!("{value:?}").contains("super-secret-value"));
}

#[tokio::test]
async fn secret_hmac_fails_closed_when_provider_is_unavailable() {
    let adapter = UnavailableSecretClient;
    let response = adapter
        .execute(SecretRequest {
            id: HostRequestId(14),
            operation: SecretOperation::HmacSha256 {
                key: etas_host::SecretRef::new("ETAS_TEST_SECRET"),
                body: b"payload".to_vec(),
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Secret", "use")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(14)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("secret response");
    assert_eq!(
        response
            .result
            .expect_err("unavailable secret client should fail closed")
            .code,
        HostErrorCode::ProviderUnavailable
    );
}

#[tokio::test]
async fn browser_screenshot_fails_closed_when_provider_is_unavailable() {
    let adapter = UnavailableBrowserProtocolClient;
    let response = adapter
        .execute(BrowserProtocolRequest {
            id: HostRequestId(15),
            operation: BrowserProtocolOperation::Screenshot {
                session: "browser-session".to_owned(),
                max_bytes: 1024,
            },
            authority: AuthorityContext {
                grants: vec![HostActionGrant::allow("Browser", "screenshot")],
                approvals: Vec::new(),
                sandbox: SandboxPolicy::deny_all(),
                policy: Default::default(),
            },
            trace: TraceContext::root(TraceId(15)),
            budget: etas_host::ExecutionBudget::default(),
        })
        .await
        .expect("browser response");
    assert_eq!(
        response
            .result
            .expect_err("unavailable browser client should fail closed")
            .code,
        HostErrorCode::ProviderUnavailable
    );
}

#[tokio::test]
async fn in_memory_client_detects_version_conflict() {
    let adapter = InMemoryMemoryClient::new();
    let store = StoreRef {
        region: etas_host::MemoryRegionRef {
            stable_id: "test".to_owned(),
            schema_fingerprint: None,
        },
        path: vec!["items".to_owned()],
    };
    let first = adapter
        .execute(memory_request(
            HostRequestId(8),
            store.clone(),
            MemoryOperation::Put {
                key: etas_host::HostValue::String("k".to_owned()),
                value: etas_host::HostValue::String("v1".to_owned()),
                expected: None,
                mode: etas_host::MemoryWriteMode::Put,
            },
        ))
        .await
        .expect("memory response");
    assert!(matches!(first.result, Ok(MemoryResult::Written { .. })));

    let stale = adapter
        .execute(memory_request(
            HostRequestId(9),
            store,
            MemoryOperation::Put {
                key: etas_host::HostValue::String("k".to_owned()),
                value: etas_host::HostValue::String("v2".to_owned()),
                expected: Some(MemoryVersion {
                    opaque: "99".to_owned(),
                }),
                mode: etas_host::MemoryWriteMode::Put,
            },
        ))
        .await
        .expect("memory response");
    assert!(matches!(stale.result, Ok(MemoryResult::Conflict(_))));
}

#[tokio::test]
async fn http_transport_rejects_endpoint_outside_adapter_authority() {
    let transport = HttpTransport::try_new(
        "http://127.0.0.1:8848",
        etas_host::PrivateResolutionPolicy::AllowPrivate,
    )
    .expect("local transport endpoint should be valid");
    let error = transport
        .send_raw(
            "GET",
            "http://127.0.0.1:8849/v1/models",
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect_err("unallowlisted endpoint should be rejected");
    assert_eq!(error.code, HostErrorCode::AuthorityDenied);
}

#[tokio::test]
async fn public_http_transport_rejects_private_endpoint_resolution() {
    let transport = HttpTransport::try_new(
        "http://127.0.0.1:8848",
        etas_host::PrivateResolutionPolicy::PublicOnly,
    )
    .expect("endpoint syntax should be valid");
    let error = transport
        .send_raw(
            "GET",
            "http://127.0.0.1:8848/v1/models",
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect_err("public transport must reject a private endpoint before connecting");
    assert_eq!(error.code, HostErrorCode::AuthorityDenied);
}

#[tokio::test]
async fn http_transport_does_not_follow_redirects_outside_adapter_authority() {
    let redirect_listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect endpoint");
    let redirect_address = redirect_listener.local_addr().expect("redirect address");
    let target_listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect target");
    let target_address = target_listener.local_addr().expect("target address");
    target_listener
        .set_nonblocking(true)
        .expect("configure redirect target");

    let target_reached = Arc::new(AtomicBool::new(false));
    let target_reached_by_server = Arc::clone(&target_reached);
    let target_server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            match target_listener.accept() {
                Ok((mut stream, _)) => {
                    target_reached_by_server.store(true, Ordering::SeqCst);
                    let mut request = [0_u8; 1024];
                    let _ = stream.read(&mut request);
                    stream
                        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
                        .expect("write redirect target response");
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("redirect target accept failed: {error}"),
            }
        }
    });

    let redirect_server = thread::spawn(move || {
        let (mut stream, _) = redirect_listener.accept().expect("accept adapter request");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        write!(
            stream,
            "HTTP/1.1 302 Found\r\nLocation: http://{target_address}/internal\r\nContent-Length: 0\r\n\r\n"
        )
        .expect("write redirect response");
    });

    let transport = HttpTransport::try_new(
        format!("http://{redirect_address}"),
        etas_host::PrivateResolutionPolicy::AllowPrivate,
    )
    .expect("loopback transport endpoint should be valid");
    let response = transport
        .send_raw(
            "GET",
            &format!("http://{redirect_address}/v1/models"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("redirect response should be returned without following it");

    redirect_server.join().expect("redirect server completed");
    target_server
        .join()
        .expect("redirect target server completed");
    assert_eq!(response.status, 302);
    assert!(!target_reached.load(Ordering::SeqCst));
}

#[tokio::test]
async fn http_transport_uses_configured_request_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed endpoint");
    let address = listener.local_addr().expect("delayed endpoint address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept delayed request");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        thread::sleep(Duration::from_millis(150));
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        );
    });
    let timeout = TransportTimeoutPolicy::try_from_millis(100, 500)
        .expect("configured deadline should be valid");
    let transport = HttpTransport::try_new(
        format!("http://{address}"),
        etas_host::PrivateResolutionPolicy::AllowPrivate,
    )
    .expect("delayed endpoint should be valid")
    .with_timeout(timeout);

    let response = transport
        .send_json("/v1/test", "{}".to_owned())
        .await
        .expect("response within configured deadline should succeed");

    server.join().expect("delayed server completed");
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn execution_deadline_tightens_transport_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind delayed endpoint");
    let address = listener.local_addr().expect("delayed endpoint address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept delayed request");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        thread::sleep(Duration::from_millis(250));
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{}",
        );
    });
    let timeout = TransportTimeoutPolicy::try_from_millis(100, 500)
        .expect("configured deadline should be valid");
    let transport = HttpTransport::try_new(
        format!("http://{address}"),
        etas_host::PrivateResolutionPolicy::AllowPrivate,
    )
    .expect("delayed endpoint should be valid")
    .with_timeout(timeout);

    let error = transport
        .send_json_with_deadline(
            "/v1/test",
            "{}".to_owned(),
            Some(tokio::time::Instant::now() + Duration::from_millis(100)),
        )
        .await
        .expect_err("execution deadline must tighten transport deadline");

    server.join().expect("delayed server completed");
    assert_eq!(error.code, HostErrorCode::TimedOut);
}

#[tokio::test]
async fn retry_delay_does_not_reset_request_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind failing endpoint");
    let address = listener.local_addr().expect("failing endpoint address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept failing request");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
    });
    let timeout = TransportTimeoutPolicy::try_from_millis(100, 100)
        .expect("configured deadline should be valid");
    let transport = HttpTransport::try_new(
        format!("http://{address}"),
        etas_host::PrivateResolutionPolicy::AllowPrivate,
    )
    .expect("failing endpoint should be valid")
    .with_timeout(timeout)
    .with_retry(RetryPolicy {
        attempts: 2,
        delay: Duration::from_millis(200),
    });
    let started = Instant::now();

    let error = transport
        .send_json("/v1/test", "{}".to_owned())
        .await
        .expect_err("retry delay must remain inside the original deadline");

    server.join().expect("failing server completed");
    assert_eq!(error.code, HostErrorCode::TimedOut);
    assert!(started.elapsed() < Duration::from_millis(300));
}

fn model_request(id: HostRequestId, sandbox: SandboxPolicy) -> ModelRequest {
    ModelRequest {
        id,
        provider: None,
        model: ModelName("sandbox-test".to_owned()),
        messages: vec![ModelMessage {
            role: ModelRole::User,
            content: vec![ModelContent::Text("hello".to_owned())],
            tool_call_id: None,
            tool_calls: Vec::new(),
        }],
        tools: Vec::new(),
        tool_choice: Default::default(),
        response_schema: None,
        policy_ref: None,
        options: ModelOptions {
            temperature: Some(0.0),
            max_output_tokens: Some(8),
            metadata: Vec::new(),
        },
        authority: AuthorityContext {
            grants: vec![HostActionGrant::allow("Agentic", "infer")],
            approvals: Vec::new(),
            sandbox,
            policy: Default::default(),
        },
        trace: TraceContext::root(TraceId(u128::from(id.0))),
        budget: etas_host::ExecutionBudget::default(),
    }
}

fn tool_request(id: HostRequestId, sandbox: SandboxPolicy) -> ToolRequest {
    ToolRequest {
        id,
        tool: ToolRef::anonymous_test("host.test"),
        args: etas_host::HostValue::String("hello".to_owned()),
        authority: AuthorityContext {
            grants: vec![HostActionGrant::allow("Tool", "host.test")],
            approvals: Vec::new(),
            sandbox,
            policy: Default::default(),
        },
        trace: TraceContext::root(TraceId(u128::from(id.0))),
        budget: etas_host::ExecutionBudget::default(),
    }
}

fn memory_request(id: HostRequestId, store: StoreRef, operation: MemoryOperation) -> MemoryRequest {
    MemoryRequest {
        id,
        store,
        operation,
        authority: AuthorityContext {
            grants: vec![HostActionGrant::allow("Memory", "write")],
            approvals: Vec::new(),
            sandbox: SandboxPolicy::deny_all(),
            policy: Default::default(),
        },
        trace: TraceContext::root(TraceId(u128::from(id.0))),
        budget: etas_host::ExecutionBudget::default(),
    }
}

fn fs_authority(root: etas_host::WorkspaceRoot) -> AuthorityContext {
    AuthorityContext {
        grants: vec![
            HostActionGrant::allow("Fs", "read"),
            HostActionGrant::allow("Fs", "write"),
        ],
        approvals: Vec::new(),
        sandbox: SandboxPolicy::allow_listed(
            etas_host::FilesystemPolicy {
                read_roots: vec![root.clone()],
                write_roots: vec![root],
                delete_roots: Vec::new(),
            },
            NetworkPolicy::deny_all(),
            etas_host::CommandPolicy::allow_programs(Vec::new()),
            etas_host::DestructiveOpPolicy::deny_all(),
        ),
        policy: Default::default(),
    }
}
