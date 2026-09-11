use super::*;

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
