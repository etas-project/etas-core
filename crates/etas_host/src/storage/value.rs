use crate::{HostError, HostErrorCode, HostValue, StorageOperationRef};

/// The portable Etas storage error ABI; operation records are data, not evidence.
pub fn storage_error_value(error: HostError, operation: Option<StorageOperationRef>) -> HostValue {
    let code = match error.code {
        HostErrorCode::InvalidRequest => "invalid_request",
        HostErrorCode::SchemaMismatch => "schema_mismatch",
        HostErrorCode::Unsupported => "unsupported",
        HostErrorCode::ProviderUnavailable | HostErrorCode::ToolUnavailable => "unavailable",
        HostErrorCode::ProviderRejected | HostErrorCode::ToolRejected => "rejected",
        HostErrorCode::TimedOut => "timed_out",
        // StorageLimits uses this host code for explicit value/page/ledger bounds.
        // Execution cancellation and scope-budget exhaustion stay control signals.
        HostErrorCode::BudgetExceeded => "limit_exceeded",
        _ => "host",
    };
    HostValue::Record(vec![
        ("code".into(), HostValue::String(code.into())),
        ("message".into(), HostValue::String(error.message)),
        (
            "operation".into(),
            match operation {
                Some(operation) => HostValue::Variant {
                    name: "Some".into(),
                    fields: vec![HostValue::Record(vec![
                        (
                            "key".into(),
                            HostValue::String(operation.key.as_str().into()),
                        ),
                        (
                            "fingerprint".into(),
                            HostValue::String(operation.request_fingerprint),
                        ),
                    ])],
                },
                None => HostValue::Variant {
                    name: "None".into(),
                    fields: vec![],
                },
            },
        ),
    ])
}
