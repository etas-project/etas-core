use super::{
    MemoryConfirmedOutcome, MemoryNotCommitted, MemoryWriteChange, MemoryWriteReceipt,
    MemoryWriteResult,
};
use crate::{
    ConfirmedOutcome, HostValue, MemoryVersion, ReceiptLookup, StorageDurability,
    StorageOperationRef, WriteOutcome,
};

#[cfg(test)]
mod tests;

/// Public storage outcome ABI. Internal diagnostics and conflict values are not payload fields.
pub fn memory_write_result_value(result: MemoryWriteResult) -> HostValue {
    match result {
        MemoryWriteResult::Outcome(WriteOutcome::Committed(receipt)) => {
            variant("Committed", vec![receipt_value(receipt)])
        }
        MemoryWriteResult::Outcome(WriteOutcome::NotCommitted { operation, reason }) => variant(
            "NotCommitted",
            vec![operation_value(operation), rejection_value(reason)],
        ),
        MemoryWriteResult::Outcome(WriteOutcome::Unknown { operation, .. }) => {
            variant("Unknown", vec![operation_value(operation)])
        }
        MemoryWriteResult::Receipt(ReceiptLookup::Found(outcome)) => {
            variant("Found", vec![confirmed_value(outcome)])
        }
        MemoryWriteResult::Receipt(ReceiptLookup::Unresolved) => variant("Unresolved", vec![]),
        MemoryWriteResult::Receipt(ReceiptLookup::Expired) => variant("Expired", vec![]),
    }
}

fn confirmed_value(outcome: MemoryConfirmedOutcome) -> HostValue {
    match outcome {
        ConfirmedOutcome::Committed(receipt) => variant("Committed", vec![receipt_value(receipt)]),
        ConfirmedOutcome::NotCommitted { operation, reason } => variant(
            "NotCommitted",
            vec![operation_value(operation), rejection_value(reason)],
        ),
    }
}

fn receipt_value(receipt: MemoryWriteReceipt) -> HostValue {
    let target = receipt.target;
    record(vec![
        ("operation", operation_value(receipt.operation)),
        (
            "target",
            record(vec![
                ("region", HostValue::String(target.store.region.stable_id)),
                (
                    "store",
                    HostValue::List(
                        target
                            .store
                            .path
                            .into_iter()
                            .map(HostValue::String)
                            .collect(),
                    ),
                ),
                (
                    "schema_fingerprint",
                    optional(
                        target
                            .store
                            .region
                            .schema_fingerprint
                            .map(HostValue::String),
                    ),
                ),
                ("key", target.key),
            ]),
        ),
        (
            "change",
            match receipt.change {
                MemoryWriteChange::Written { version } => {
                    variant("Written", vec![version_value(version)])
                }
                MemoryWriteChange::Deleted { tombstone } => {
                    variant("Deleted", vec![version_value(tombstone)])
                }
            },
        ),
        (
            "durability",
            variant(
                match receipt.durability {
                    StorageDurability::Volatile => "Volatile",
                    StorageDurability::SqliteWalFull => "SqliteWalFull",
                },
                vec![],
            ),
        ),
    ])
}

fn rejection_value(reason: MemoryNotCommitted) -> HostValue {
    match reason {
        MemoryNotCommitted::Conflict {
            expected, actual, ..
        } => variant(
            "ConditionConflict",
            vec![
                optional(expected.map(version_value)),
                optional(actual.map(version_value)),
            ],
        ),
        MemoryNotCommitted::Unchanged => variant("Unchanged", vec![]),
        MemoryNotCommitted::Rejected(error) => variant(
            "Rejected",
            vec![record(vec![
                ("code", HostValue::String(error.code.as_str().into())),
                ("message", HostValue::String(error.message)),
            ])],
        ),
    }
}

fn operation_value(operation: StorageOperationRef) -> HostValue {
    record(vec![
        ("key", HostValue::String(operation.key.as_str().into())),
        (
            "fingerprint",
            HostValue::String(operation.request_fingerprint),
        ),
    ])
}
fn version_value(version: MemoryVersion) -> HostValue {
    record(vec![(
        "opaque",
        HostValue::String(version.as_token().into()),
    )])
}
fn optional(value: Option<HostValue>) -> HostValue {
    match value {
        Some(value) => variant("Some", vec![value]),
        None => variant("None", vec![]),
    }
}
fn variant(name: &str, fields: Vec<HostValue>) -> HostValue {
    HostValue::Variant {
        name: name.into(),
        fields,
    }
}
fn record(fields: Vec<(&str, HostValue)>) -> HostValue {
    HostValue::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect(),
    )
}
