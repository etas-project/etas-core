use super::{SessionContextEvidence, SessionContextReceipt, SessionContextRejection};
use crate::session::SessionWriteResult;
use crate::{
    HostError, HostErrorCode, HostValue, ReceiptLookup, StorageDurability, StorageOperationRef,
    WriteOutcome,
};

pub fn session_context_result_value(result: SessionWriteResult) -> Result<HostValue, HostError> {
    Ok(match result {
        SessionWriteResult::Context(WriteOutcome::Committed(receipt)) => {
            variant("Committed", vec![receipt_value(receipt)])
        }
        SessionWriteResult::Context(WriteOutcome::NotCommitted { operation, reason }) => {
            rejected(operation, reason)
        }
        SessionWriteResult::Context(WriteOutcome::Unknown { operation, .. }) => {
            variant("Unknown", vec![operation_value(operation)])
        }
        SessionWriteResult::ContextReceipt(ReceiptLookup::Found(evidence)) => variant(
            "Found",
            vec![match evidence {
                SessionContextEvidence::Committed(receipt) => {
                    variant("Committed", vec![receipt_value(receipt)])
                }
                SessionContextEvidence::NotCommitted { operation, reason } => {
                    rejected(operation, reason)
                }
            }],
        ),
        SessionWriteResult::ContextReceipt(ReceiptLookup::Unresolved) => {
            variant("Unresolved", vec![])
        }
        SessionWriteResult::ContextReceipt(ReceiptLookup::Expired) => variant("Expired", vec![]),
        _ => {
            return Err(HostError::new(
                HostErrorCode::InvalidResponse,
                "expected session context outcome",
            ));
        }
    })
}
fn receipt_value(receipt: SessionContextReceipt) -> HostValue {
    HostValue::Record(vec![
        ("operation".into(), operation_value(receipt.operation)),
        ("session".into(), HostValue::String(receipt.session.id)),
        (
            "generation".into(),
            HostValue::String(receipt.generation.as_token().into()),
        ),
        (
            "context_version".into(),
            HostValue::UInt(receipt.context_version.into()),
        ),
        (
            "durability".into(),
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
fn rejected(operation: StorageOperationRef, reason: SessionContextRejection) -> HostValue {
    variant(
        "NotCommitted",
        vec![
            operation_value(operation),
            match reason {
                SessionContextRejection::StaleHistory => variant("StaleHistory", vec![]),
                SessionContextRejection::Rejected(error) => variant(
                    "Rejected",
                    vec![HostValue::Record(vec![
                        ("code".into(), HostValue::String(error.code.as_str().into())),
                        ("message".into(), HostValue::String(error.message)),
                    ])],
                ),
            },
        ],
    )
}
fn operation_value(operation: StorageOperationRef) -> HostValue {
    HostValue::Record(vec![
        (
            "key".into(),
            HostValue::String(operation.key.as_str().into()),
        ),
        (
            "fingerprint".into(),
            HostValue::String(operation.request_fingerprint),
        ),
    ])
}
fn variant(name: &str, fields: Vec<HostValue>) -> HostValue {
    HostValue::Variant {
        name: name.into(),
        fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StorageOperationKey;

    fn operation() -> StorageOperationRef {
        StorageOperationRef {
            key: StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap(),
            request_fingerprint: "a".repeat(64),
        }
    }

    #[test]
    fn unknown_context_exposes_only_query_identity_not_backend_error() {
        let operation = operation();
        let value =
            session_context_result_value(SessionWriteResult::Context(WriteOutcome::Unknown {
                operation: operation.clone(),
                error: HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "private backend path and caller context",
                ),
            }))
            .unwrap();
        assert_eq!(value, variant("Unknown", vec![operation_value(operation)]));
        assert!(!format!("{value:?}").contains("private backend"));
    }

    #[test]
    fn context_reconciliation_retains_rejection_and_distinguishes_unresolved_from_expired() {
        let operation = operation();
        let evidence = SessionContextEvidence::NotCommitted {
            operation: operation.clone(),
            reason: SessionContextRejection::StaleHistory,
        };
        assert_eq!(
            session_context_result_value(SessionWriteResult::ContextReceipt(ReceiptLookup::Found(
                evidence
            ),))
            .unwrap(),
            variant(
                "Found",
                vec![rejected(operation, SessionContextRejection::StaleHistory)],
            ),
        );
        for (lookup, name) in [
            (ReceiptLookup::Unresolved, "Unresolved"),
            (ReceiptLookup::Expired, "Expired"),
        ] {
            assert_eq!(
                session_context_result_value(SessionWriteResult::ContextReceipt(lookup)).unwrap(),
                variant(name, vec![]),
            );
        }
    }

    #[test]
    fn context_codec_rejects_other_session_operation_results() {
        for result in [
            SessionWriteResult::Receipt(ReceiptLookup::Unresolved),
            SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                operation: operation(),
                reason: HostError::new(HostErrorCode::InvalidRequest, "not a publication"),
            }),
        ] {
            assert_eq!(
                session_context_result_value(result).unwrap_err().code,
                HostErrorCode::InvalidResponse,
            );
        }
    }
}
