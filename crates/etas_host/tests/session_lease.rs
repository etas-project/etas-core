use std::time::Duration;

use etas_host::{
    AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId,
    InMemorySessionClient, SessionClient, SessionOperation, SessionRef, SessionRequest,
    SessionResult, SqliteSessionClient, TestWorkspace, TraceContext, TraceId,
    session::{SessionLeaseOperation as Op, SessionLeaseResult as LeaseResult, SessionLeaseScope},
};

async fn execute(
    client: &impl SessionClient<Error = HostError>,
    operation: Op,
) -> Result<LeaseResult, HostError> {
    let response = client
        .execute(SessionRequest {
            id: HostRequestId(1),
            operation: SessionOperation::Lease(operation),
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(1)),
            budget: ExecutionBudget::default(),
        })
        .await?;
    let SessionResult::Lease(result) = response.result? else {
        panic!("lease response");
    };
    Ok(result)
}

async fn verify(
    owner: &impl SessionClient<Error = HostError>,
    peer: &impl SessionClient<Error = HostError>,
) {
    let scope = || {
        SessionLeaseScope::new(
            SessionRef {
                id: "routing".into(),
            },
            Duration::from_secs(60),
        )
        .unwrap()
    };
    let cancelled = scope();
    assert_eq!(
        execute(owner, Op::Revoke(cancelled.clone())).await.unwrap(),
        LeaseResult::Revoked,
    );
    assert_eq!(
        execute(peer, Op::Acquire(cancelled))
            .await
            .unwrap_err()
            .code,
        HostErrorCode::ProviderRejected,
    );
    let first_scope = scope();
    let competitor = scope();
    let LeaseResult::Acquired(first) = execute(owner, Op::Acquire(first_scope.clone()))
        .await
        .unwrap()
    else {
        panic!("lease acquisition");
    };
    assert_eq!(
        execute(peer, Op::Acquire(competitor.clone()))
            .await
            .unwrap_err()
            .code,
        HostErrorCode::ProviderRejected,
    );
    let LeaseResult::Transferred(next) = execute(peer, Op::Handoff(first.clone())).await.unwrap()
    else {
        panic!("lease transfer");
    };
    assert_eq!(next.session(), first.session());
    assert_ne!(next.generation(), first.generation());
    assert_eq!(
        execute(owner, Op::Handoff(first.clone()))
            .await
            .unwrap_err()
            .code,
        HostErrorCode::ProviderRejected,
    );
    assert_eq!(
        execute(owner, Op::Release(first)).await.unwrap_err().code,
        HostErrorCode::ProviderRejected,
    );

    // The scope can revoke the committed transfer even if the owner missed its reply.
    assert_eq!(
        execute(owner, Op::Revoke(first_scope.clone()))
            .await
            .unwrap(),
        LeaseResult::Revoked
    );
    let LeaseResult::Acquired(replacement) = execute(peer, Op::Acquire(competitor)).await.unwrap()
    else {
        panic!("replacement acquisition");
    };
    assert_eq!(
        execute(owner, Op::Revoke(first_scope)).await.unwrap(),
        LeaseResult::Revoked
    );
    assert_eq!(
        execute(peer, Op::Release(replacement)).await.unwrap(),
        LeaseResult::Released
    );
}

#[tokio::test]
async fn session_lease_generation_and_scope_survive_backend_client_boundaries() {
    let memory = InMemorySessionClient::new();
    verify(&memory, &memory.clone()).await;

    let workspace = TestWorkspace::create("session-lease").unwrap();
    let path = workspace.path().join("session.sqlite");
    let owner = SqliteSessionClient::open(&path).unwrap();
    let peer = SqliteSessionClient::open(&path).unwrap();
    verify(&owner, &peer).await;
}

#[tokio::test]
async fn cancelled_resource_scope_revokes_the_transferred_lease() {
    use etas_host::execution::{CancellationReason, ExecutionScope, ExternalOutcome};

    let workspace = TestWorkspace::create("managed-session-lease").unwrap();
    let client = SqliteSessionClient::open(workspace.path().join("session.sqlite")).unwrap();
    let scope = ExecutionScope::new_owned();
    let registration = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    registration.begin_dispatch().unwrap();
    let context = registration
        .context()
        .clone()
        .retain_resources_in_scope(&scope);
    let session = SessionRef {
        id: "routing".into(),
    };
    let acquired = client
        .execute_scoped(
            SessionRequest {
                id: HostRequestId(1),
                operation: SessionOperation::Lease(Op::Acquire(
                    SessionLeaseScope::new(session.clone(), Duration::from_secs(60)).unwrap(),
                )),
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(1)),
                budget: ExecutionBudget::default(),
            },
            &context,
        )
        .await
        .unwrap();
    let SessionResult::Lease(LeaseResult::Acquired(lease)) = acquired.result.unwrap() else {
        panic!("lease acquisition");
    };
    registration
        .complete(ExternalOutcome::Confirmed, vec![])
        .unwrap();
    assert!(matches!(
        execute(&client, Op::Handoff(lease)).await.unwrap(),
        LeaseResult::Transferred(_)
    ));
    scope
        .cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    scope.finish_body(false).unwrap();
    scope.join().await.unwrap();
    let replacement = SessionLeaseScope::new(session, Duration::from_secs(60)).unwrap();
    assert!(matches!(
        execute(&client, Op::Acquire(replacement)).await.unwrap(),
        LeaseResult::Acquired(_)
    ));
}
