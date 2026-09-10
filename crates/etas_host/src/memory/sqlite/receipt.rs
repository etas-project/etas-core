use super::*;
use crate::memory::{
    MemoryNotCommitted, MemoryWriteChange, MemoryWriteOperation, MemoryWriteOutcome,
    MemoryWriteReceipt, MemoryWriteResult, MemoryWriteTarget,
};
use crate::storage::{
    outcome::{ConfirmedOutcome, ReceiptLookup, StorageDurability, WriteOutcome},
    receipt::StorageOperationRef,
};

impl MemoryDatabase<'_> {
    pub(super) fn execute_write(
        &mut self,
        store: &StoreRef,
        request: MemoryWriteOperation,
    ) -> Result<MemoryWriteResult, HostError> {
        match request {
            MemoryWriteOperation::Reconcile { operation } => {
                operation.validate()?;
                if operation.key.is_expired()? {
                    return Ok(MemoryWriteResult::Receipt(ReceiptLookup::Expired));
                }
                let receipt = lookup(self.connection, store, &operation, self.limits)?;
                Ok(MemoryWriteResult::Receipt(match receipt {
                    Some(receipt) => ReceiptLookup::Found(receipt),
                    None => ReceiptLookup::Unresolved,
                }))
            }
            MemoryWriteOperation::Mutate { key, mutation } => {
                let target = mutation.target(store);
                let operation = mutation.operation_ref(store, key, self.limits)?;
                operation
                    .key
                    .validate_window(self.limits.max_receipt_retention_seconds)?;
                let staged = crate::storage::sqlite::write_transaction(
                    self.connection,
                    operation.clone(),
                    |tx| {
                        match lookup(tx, store, &operation, self.limits) {
                            Ok(Some(existing)) => return Ok(existing.into()),
                            Ok(None) => {}
                            Err(error) if error.code == HostErrorCode::InvalidRequest => {
                                return Err(error);
                            }
                            Err(error) => {
                                return Ok(WriteOutcome::Unknown {
                                    operation: operation.clone(),
                                    error,
                                });
                            }
                        }
                        tx.execute(
                            "DELETE FROM memory_receipts WHERE expires<=?1",
                            [crate::storage::receipt::now()? as i64],
                        )
                        .map_err(sqlite_error)?;
                        let maximum = i64::try_from(self.limits.max_receipts)
                            .map_err(|_| crate::memory::write::limit_error())?;
                        let count: i64 = tx
                            .query_row(
                                "SELECT COUNT(*) FROM (SELECT 1 FROM memory_receipts LIMIT ?1)",
                                [maximum],
                                |row| row.get(0),
                            )
                            .map_err(sqlite_error)?;
                        if count >= maximum {
                            return Err(crate::memory::write::limit_error());
                        }
                        let used: i64 = tx.query_row(
                        "SELECT COALESCE(SUM(?1 + 2 * (length(CAST(region AS BLOB)) + length(CAST(path AS BLOB)) + length(CAST(operation AS BLOB)) + length(CAST(fingerprint AS BLOB)) + COALESCE(length(CAST(key_json AS BLOB)),0) + COALESCE(length(CAST(schema_fingerprint AS BLOB)),0))), 0) FROM memory_receipts",
                        [crate::storage::receipt_budget::FIXED_BYTES as i64], |row| row.get(0)
                    ).map_err(sqlite_error)?;
                        let path = store_path_key(store)?;
                        let key =
                            crate::value::tagged::encode_with_limits(&target.key, self.limits)?;
                        let charge = crate::storage::receipt_budget::charge([
                            store.region.stable_id.as_str(),
                            path.as_str(),
                            operation.key.as_str(),
                            operation.request_fingerprint.as_str(),
                            key.as_str(),
                            store.region.schema_fingerprint.as_deref().unwrap_or(""),
                        ])?;
                        crate::storage::receipt_budget::admit(
                            self.limits,
                            usize::try_from(used)
                                .map_err(|_| crate::storage::receipt_budget::exceeded())?,
                            charge,
                        )?;
                        let result =
                            super::write::apply_mutation(tx, store, mutation, self.limits)?;
                        let outcome = match result {
                            MemoryResult::Written { version } => {
                                WriteOutcome::Committed(MemoryWriteReceipt {
                                    operation: operation.clone(),
                                    target,
                                    change: MemoryWriteChange::Written { version },
                                    durability: StorageDurability::SqliteWalFull,
                                })
                            }
                            MemoryResult::Deleted { version } => {
                                WriteOutcome::Committed(MemoryWriteReceipt {
                                    operation: operation.clone(),
                                    target,
                                    change: MemoryWriteChange::Deleted { tombstone: version },
                                    durability: StorageDurability::SqliteWalFull,
                                })
                            }
                            MemoryResult::Conflict(conflict) => WriteOutcome::NotCommitted {
                                operation: operation.clone(),
                                reason: MemoryNotCommitted::Conflict {
                                    expected: conflict.expected,
                                    actual: conflict.actual,
                                    current_value: conflict.current_value,
                                },
                            },
                            MemoryResult::Unchanged => WriteOutcome::NotCommitted {
                                operation: operation.clone(),
                                reason: MemoryNotCommitted::Unchanged,
                            },
                            _ => return Err(invalid_receipt()),
                        };
                        persist(tx, store, &outcome, self.limits)?;
                        if let Some(context) = self.operation {
                            context.check()?;
                        }
                        Ok(outcome)
                    },
                );
                let outcome = match staged {
                    WriteOutcome::Committed(outcome) => outcome,
                    WriteOutcome::NotCommitted { operation, reason } => rejected(operation, reason),
                    WriteOutcome::Unknown { operation, error } => {
                        WriteOutcome::Unknown { operation, error }
                    }
                };
                Ok(MemoryWriteResult::Outcome(outcome))
            }
        }
    }
}

fn rejected(operation: StorageOperationRef, error: HostError) -> MemoryWriteOutcome {
    WriteOutcome::NotCommitted {
        operation,
        reason: MemoryNotCommitted::Rejected(error),
    }
}

fn persist(
    connection: &Connection,
    store: &StoreRef,
    outcome: &MemoryWriteOutcome,
    limits: &crate::StorageLimits,
) -> Result<(), HostError> {
    let (operation, kind, version, expected, actual) = match outcome {
        WriteOutcome::Committed(receipt) => (
            &receipt.operation,
            match receipt.change {
                MemoryWriteChange::Written { .. } => "put",
                MemoryWriteChange::Deleted { .. } => "delete",
            },
            Some(receipt.change.revision().as_token()),
            None,
            None,
        ),
        WriteOutcome::NotCommitted {
            operation,
            reason:
                MemoryNotCommitted::Conflict {
                    expected,
                    actual,
                    current_value: None,
                },
        } => (
            operation,
            "conflict",
            None,
            expected.as_ref().map(|v| v.as_token()),
            actual.as_ref().map(|v| v.as_token()),
        ),
        WriteOutcome::NotCommitted {
            operation,
            reason: MemoryNotCommitted::Unchanged,
        } => (operation, "unchanged", None, None, None),
        _ => return Err(invalid_receipt()),
    };
    let key = match outcome {
        WriteOutcome::Committed(receipt) => Some(crate::value::tagged::encode_with_limits(
            &receipt.target.key,
            limits,
        )?),
        _ => None,
    };
    connection.execute("INSERT INTO memory_receipts(region,path,operation,expires,fingerprint,kind,version,expected,actual,key_json,schema_fingerprint) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        rusqlite::params![store.region.stable_id, store_path_key(store)?, operation.key.as_str(), operation.key.expires_at() as i64,
            operation.request_fingerprint, kind, version, expected, actual, key, store.region.schema_fingerprint]).map_err(sqlite_error)?;
    Ok(())
}

fn lookup(
    connection: &Connection,
    store: &StoreRef,
    operation: &StorageOperationRef,
    limits: &crate::StorageLimits,
) -> Result<Option<crate::memory::MemoryConfirmedOutcome>, HostError> {
    use rusqlite::OptionalExtension;
    let stored = connection.query_row("SELECT fingerprint,kind,version,expected,actual,key_json,schema_fingerprint FROM memory_receipts WHERE region=?1 AND path=?2 AND operation=?3",
        rusqlite::params![store.region.stable_id, store_path_key(store)?, operation.key.as_str()], |row| {
            let mut total_bytes = 0usize;
            for column in 0..7 {
                let maximum = if column >= 5 { limits.max_value_bytes.min(limits.max_result_bytes) } else { 256 };
                if let rusqlite::types::ValueRef::Text(value) = row.get_ref(column)? {
                    total_bytes = total_bytes.checked_add(value.len()).ok_or(rusqlite::Error::InvalidQuery)?;
                    if value.len() > maximum || total_bytes > limits.max_result_bytes { return Err(rusqlite::Error::InvalidQuery); }
                }
            }
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,Option<String>>(2)?, row.get::<_,Option<String>>(3)?, row.get::<_,Option<String>>(4)?, row.get::<_,Option<String>>(5)?, row.get::<_,Option<String>>(6)?))
        }).optional().map_err(sqlite_error)?;
    let Some((fingerprint, kind, version, expected, actual, key, schema_fingerprint)) = stored
    else {
        return Ok(None);
    };
    if fingerprint != operation.request_fingerprint {
        return Err(crate::memory::write::mismatch());
    }
    if matches!(kind.as_str(), "put" | "delete") && key.is_none() {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "legacy memory receipt lacks target evidence; replay is unsafe",
        ));
    }
    if schema_fingerprint != store.region.schema_fingerprint {
        return Err(crate::memory::write::mismatch());
    }
    let operation = operation.clone();
    let parse = |v: String| crate::MemoryVersion::parse(&v).map_err(|_| invalid_receipt());
    let result = match (kind.as_str(), version, expected, actual) {
        ("put" | "delete", Some(version), None, None) => {
            let key = key.ok_or_else(|| {
                HostError::new(
                    HostErrorCode::SchemaMismatch,
                    "legacy memory receipt lacks target evidence; replay is unsafe",
                )
            })?;
            let key = crate::value::tagged::decode_with_limits(&key, limits)?;
            let version = parse(version)?;
            ConfirmedOutcome::Committed(MemoryWriteReceipt {
                operation,
                target: MemoryWriteTarget {
                    store: store.clone(),
                    key,
                },
                change: if kind == "put" {
                    MemoryWriteChange::Written { version }
                } else {
                    MemoryWriteChange::Deleted { tombstone: version }
                },
                durability: StorageDurability::SqliteWalFull,
            })
        }
        ("conflict", None, expected, actual) => ConfirmedOutcome::NotCommitted {
            operation,
            reason: MemoryNotCommitted::Conflict {
                expected: expected.map(parse).transpose()?,
                actual: actual.map(parse).transpose()?,
                current_value: None,
            },
        },
        ("unchanged", None, None, None) => ConfirmedOutcome::NotCommitted {
            operation,
            reason: MemoryNotCommitted::Unchanged,
        },
        _ => return Err(invalid_receipt()),
    };
    Ok(Some(result))
}
fn invalid_receipt() -> HostError {
    HostError::new(
        HostErrorCode::SchemaMismatch,
        "invalid stored memory receipt",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMutation;
    use crate::{MemoryRegionRef, StorageLimits, StorageOperationKey, WriteCondition};

    #[test]
    fn commit_failure_is_unknown_and_reconciliation_never_claims_absence_is_rollback() {
        let workspace = crate::TestWorkspace::create("receipt-commit-failure").unwrap();
        let mut connection = Connection::open(workspace.path().join("db")).unwrap();
        super::super::schema::initialize(&mut connection).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE commit_parent(id INTEGER PRIMARY KEY);
            CREATE TABLE commit_child(parent INTEGER REFERENCES commit_parent(id) DEFERRABLE INITIALLY DEFERRED);
            CREATE TRIGGER fail_commit AFTER INSERT ON memory_receipts BEGIN INSERT INTO commit_child VALUES(1); END;").unwrap();
        let limits = StorageLimits::default();
        let store = StoreRef {
            region: MemoryRegionRef {
                stable_id: "commit-error".into(),
                schema_fingerprint: None,
            },
            path: vec![],
        };
        let mut db = MemoryDatabase {
            connection: &mut connection,
            operation: None,
            limits: &limits,
        };
        let result = db
            .execute_write(
                &store,
                MemoryWriteOperation::Mutate {
                    key: StorageOperationKey::new(std::time::Duration::from_secs(3600)).unwrap(),
                    mutation: MemoryMutation::Put {
                        key: HostValue::String("key".into()),
                        value: HostValue::String("value".into()),
                        condition: WriteCondition::Any,
                    },
                },
            )
            .unwrap();
        let MemoryWriteResult::Outcome(WriteOutcome::Unknown { operation, .. }) = result else {
            panic!("commit failure must preserve uncertainty")
        };
        assert_eq!(
            db.execute_write(&store, MemoryWriteOperation::Reconcile { operation })
                .unwrap(),
            MemoryWriteResult::Receipt(ReceiptLookup::Unresolved)
        );
    }
}
