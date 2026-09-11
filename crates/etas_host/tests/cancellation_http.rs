use etas_host::{
    HostErrorCode, HostRequestId, HttpTransport, PrivateResolutionPolicy, TraceContext, TraceId,
    execution::{CancellationReason, ExecutionScope, ExternalOutcome},
    transport::RetryPolicy,
};
use std::time::Duration;
use tokio::{io::AsyncReadExt, net::TcpListener, time::timeout};

#[tokio::test(flavor = "current_thread")]
async fn cancellation_closes_http_wait_without_retrying_unknown_remote_work() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let transport = HttpTransport::try_new(
        format!("http://{address}"),
        PrivateResolutionPolicy::AllowPrivate,
    )
    .unwrap()
    .with_retry(RetryPolicy {
        attempts: 3,
        delay: Duration::from_millis(1),
    });
    let scope = ExecutionScope::new();
    let registration = scope
        .register(None, Some(HostRequestId(1)), TraceContext::root(TraceId(1)))
        .unwrap();
    registration.begin_dispatch().unwrap();
    let context = registration.context().clone();
    let call = tokio::spawn(async move {
        transport
            .send_json_scoped("/model", "{}".into(), None, &context)
            .await
    });
    let (mut peer, _) = timeout(Duration::from_secs(1), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut bytes = [0; 4096];
    let count = timeout(Duration::from_secs(1), peer.read(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert!(count > 0, "request must be sent before cancellation");
    scope
        .cancel_source()
        .stop(CancellationReason::Requested)
        .unwrap();
    let error = timeout(Duration::from_secs(1), call)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_eq!(error.code, HostErrorCode::Cancelled);
    registration
        .complete(ExternalOutcome::Unknown, vec![])
        .unwrap();
    scope.finish_body(true).unwrap();
    let report = scope.join().await.unwrap();
    assert_eq!(
        report.operations().len(),
        2,
        "one dispatch and exactly one HTTP attempt"
    );
    assert!(
        report
            .operations()
            .iter()
            .all(|op| op.outcome() == Some(&ExternalOutcome::Unknown))
    );
    assert!(
        timeout(Duration::from_millis(100), listener.accept())
            .await
            .is_err(),
        "cancelled call must not retry"
    );
}
