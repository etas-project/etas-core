use etas_host::{
    AuthorityContext, CommandClient, CommandOutput, CommandPolicy, CommandRequest,
    DestructiveOpPolicy, FilesystemPolicy, HostActionGrant, HostErrorCode, HostRequestId,
    LocalCommandClient, NetworkPolicy, PolicyContext, SandboxPolicy, TraceContext, TraceId,
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
                CommandPolicy::allow_programs(vec![program.to_owned()]),
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
                CommandPolicy::allow_programs(vec![program.to_owned()]),
                DestructiveOpPolicy::deny_all(),
            ),
        ))
        .await
        .expect("allowlisted command should execute");
    let CommandOutput {
        exit_code,
        stdout,
        stderr,
    } = response.result.expect("command should return output");

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, b"etas\n");
    assert!(stderr.is_empty());
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
