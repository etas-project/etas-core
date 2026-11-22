use etas_host::{HostError, HostErrorCode};

pub fn assert_writer_lock(error: &HostError) {
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
    assert!(
        error
            .details
            .iter()
            .any(|detail| { detail.key == "error" && detail.value == "database is locked" }),
        "expected bounded SQLite writer-lock contention: {error:?}"
    );
}
