use crate::{HostError, HostErrorCode, StorageOperationRef, WriteOutcome};
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub(crate) fn write_transaction<R>(
    connection: &mut Connection,
    operation: StorageOperationRef,
    stage: impl FnOnce(&Transaction<'_>) -> Result<R, HostError>,
) -> WriteOutcome<R, HostError> {
    let transaction = match connection.transaction_with_behavior(TransactionBehavior::Immediate) {
        Ok(transaction) => transaction,
        Err(error) => {
            return WriteOutcome::NotCommitted {
                operation,
                reason: sqlite_error(error),
            };
        }
    };
    match stage(&transaction) {
        Ok(receipt) => match transaction.commit() {
            Ok(()) => WriteOutcome::Committed(receipt),
            Err(error) => WriteOutcome::Unknown {
                operation,
                error: sqlite_error(error),
            },
        },
        Err(reason) => match transaction.rollback() {
            Ok(()) => WriteOutcome::NotCommitted { operation, reason },
            Err(error) => WriteOutcome::Unknown {
                operation,
                error: sqlite_error(error).with_detail("staging_error", reason.message),
            },
        },
    }
}

fn sqlite_error(error: rusqlite::Error) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "SQLite storage transaction failed",
    )
    .with_detail("error", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn operation() -> StorageOperationRef {
        StorageOperationRef {
            key: crate::StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap(),
            request_fingerprint: "0".repeat(64),
        }
    }

    #[test]
    fn confirmed_rollback_does_not_publish_staged_mutation() {
        let mut db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE data(value INTEGER)")
            .unwrap();
        let result: WriteOutcome<(), HostError> = write_transaction(&mut db, operation(), |tx| {
            tx.execute("INSERT INTO data VALUES(1)", []).unwrap();
            Err(HostError::new(
                HostErrorCode::BudgetExceeded,
                "cancel before commit",
            ))
        });
        assert!(matches!(
            result,
            WriteOutcome::NotCommitted {
                reason: HostError {
                    code: HostErrorCode::BudgetExceeded,
                    ..
                },
                ..
            }
        ));
        assert_eq!(
            db.query_row("SELECT count(*) FROM data", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn failed_rollback_cannot_claim_non_commit() {
        let mut db = Connection::open_in_memory().unwrap();
        let result: WriteOutcome<(), HostError> = write_transaction(&mut db, operation(), |tx| {
            tx.execute_batch("ROLLBACK").unwrap();
            Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "transaction unexpectedly ended",
            ))
        });
        assert!(matches!(result, WriteOutcome::Unknown { .. }));
    }
}
