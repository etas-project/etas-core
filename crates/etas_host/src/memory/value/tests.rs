use super::*;
use crate::{HostError, HostErrorCode, StorageOperationKey};

fn operation() -> StorageOperationRef {
    StorageOperationRef {
        key: StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap(),
        request_fingerprint: "a".repeat(64),
    }
}

#[test]
fn public_conflict_abi_does_not_expose_the_previous_value() {
    let operation = operation();
    let value = memory_write_result_value(MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
        operation: operation.clone(),
        reason: MemoryNotCommitted::Conflict {
            expected: None,
            actual: None,
            current_value: Some(HostValue::String("private prior value".into())),
        },
    }));
    assert_eq!(
        value,
        variant(
            "NotCommitted",
            vec![
                operation_value(operation),
                variant("ConditionConflict", vec![optional(None), optional(None)]),
            ]
        )
    );
}

#[test]
fn public_unknown_abi_preserves_operation_without_internal_diagnostics() {
    let operation = operation();
    let value = memory_write_result_value(MemoryWriteResult::Outcome(WriteOutcome::Unknown {
        operation: operation.clone(),
        error: HostError::new(
            HostErrorCode::ProviderUnavailable,
            "private backend diagnostic",
        ),
    }));
    assert_eq!(value, variant("Unknown", vec![operation_value(operation)]));
}

#[test]
fn missing_and_expired_receipt_evidence_remain_distinct() {
    assert_eq!(
        memory_write_result_value(MemoryWriteResult::Receipt(ReceiptLookup::Unresolved)),
        variant("Unresolved", vec![])
    );
    assert_eq!(
        memory_write_result_value(MemoryWriteResult::Receipt(ReceiptLookup::Expired)),
        variant("Expired", vec![])
    );
}
