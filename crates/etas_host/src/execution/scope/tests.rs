use super::*;

#[tokio::test]
async fn owned_resources_release_in_reverse_order_after_children_finish() {
    let scope = ExecutionScope::new_owned();
    let first = scope
        .register(None, None, TraceContext::root(crate::TraceId(1)))
        .unwrap();
    let second = scope
        .register(None, None, TraceContext::root(crate::TraceId(2)))
        .unwrap();
    let first_id = first.context().operation();
    let second_id = second.context().operation();
    scope.mark_resource(first_id).unwrap();
    scope.mark_resource(second_id).unwrap();
    first.begin_dispatch().unwrap();
    second.begin_dispatch().unwrap();
    let child = scope.child().unwrap();
    child.claim_body().unwrap();
    scope.finish_body(false).unwrap();
    assert!(!scope.resource_release_ready(second_id).unwrap());
    child.finish_body(true).unwrap();
    assert!(scope.resource_release_ready(second_id).unwrap());
    assert!(!scope.resource_release_ready(first_id).unwrap());
    second.complete(ExternalOutcome::Confirmed, vec![]).unwrap();
    assert!(scope.resource_release_ready(first_id).unwrap());
    first.complete(ExternalOutcome::Confirmed, vec![]).unwrap();
    assert_eq!(scope.join().await.unwrap().operations().len(), 2);
}

#[test]
fn poisoned_registry_is_a_host_error_not_a_panic() {
    let scope = ExecutionScope::new();
    let poison = scope.clone();
    let _ = std::thread::spawn(move || {
        let _lock = poison.tree.registry.lock().unwrap();
        panic!("inject registry poisoning");
    })
    .join();
    let error = scope.state().unwrap_err();
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
    assert!(
        scope
            .cancel_source()
            .stop(CancellationReason::Requested)
            .is_err()
    );
}
