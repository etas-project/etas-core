use super::*;
use crate::session::{
    SessionAppendReceipt, SessionResolveReceipt, SessionVersion, SessionWriteOperation,
    SessionWriteReceipt, SessionWriteResult, append_operation_ref, resolve_operation_ref,
};
use crate::{ReceiptLookup, StorageDurability, StorageOperationRef, WriteOutcome};

impl SessionDatabase<'_> {
    pub(super) fn execute_write(
        &mut self,
        request: SessionWriteOperation,
    ) -> Result<SessionWriteResult, HostError> {
        match request {
            SessionWriteOperation::PublishContext(publication) => {
                super::context::publish(self, *publication)
            }
            SessionWriteOperation::ReconcileContext { session, operation } => {
                super::context::reconcile(self.connection, &session, &operation)
            }
            SessionWriteOperation::Resolve { key, config } => {
                let operation = resolve_operation_ref(&config, key, self.limits)?;
                if let Err(reason) = operation
                    .key
                    .validate_window(self.limits.max_receipt_retention_seconds)
                {
                    return Ok(SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                        operation,
                        reason,
                    }));
                }
                let outcome = crate::storage::sqlite::write_transaction(
                    self.connection,
                    operation.clone(),
                    |tx| {
                        let session = SessionRef {
                            id: config.id.clone(),
                        };
                        if let Some(existing) = lookup(tx, &session, &operation, self.limits)? {
                            return Ok(existing);
                        }
                        let used = capacity(tx, self.limits)?;
                        let SessionResult::Resolved { created, .. } =
                            super::append::apply_resolve(tx, config, self.limits)?
                        else {
                            return Err(invalid_receipt());
                        };
                        let generation = storage_generation(tx, &session.id)?;
                        let receipt = SessionWriteReceipt::Resolve(SessionResolveReceipt {
                            operation: operation.clone(),
                            session: session.clone(),
                            created,
                            generation: crate::session::SessionGeneration::issue(
                                &session.id,
                                &generation,
                            )?,
                            durability: StorageDurability::SqliteWalFull,
                        });
                        crate::storage::receipt_budget::admit(
                            self.limits,
                            used,
                            receipt.charge(&session.id)?,
                        )?;
                        tx.execute("INSERT INTO session_receipts(session_id,operation,expires,fingerprint,kind,generation,created)
                        VALUES(?1,?2,?3,?4,'resolve',?5,?6)", params![session.id, operation.key.as_str(), operation.key.expires_at() as i64, operation.request_fingerprint, generation, created]).map_err(sqlite_error)?;
                        if let Some(context) = self.operation {
                            context.check()?;
                        }
                        Ok(receipt)
                    },
                );
                Ok(SessionWriteResult::Outcome(outcome))
            }
            SessionWriteOperation::Reconcile { session, operation } => {
                operation.validate()?;
                if operation.key.is_expired()? {
                    return Ok(SessionWriteResult::Receipt(ReceiptLookup::Expired));
                }
                Ok(SessionWriteResult::Receipt(
                    match lookup(self.connection, &session, &operation, self.limits)? {
                        Some(receipt) => ReceiptLookup::Found(receipt),
                        None => ReceiptLookup::Unresolved,
                    },
                ))
            }
            SessionWriteOperation::Append { key, message } => {
                let operation = append_operation_ref(&message, key, self.limits)?;
                let reject = |reason| {
                    SessionWriteResult::Outcome(WriteOutcome::NotCommitted {
                        operation: operation.clone(),
                        reason,
                    })
                };
                if let Err(error) = operation
                    .key
                    .validate_window(self.limits.max_receipt_retention_seconds)
                {
                    return Ok(reject(error));
                }
                let outcome = crate::storage::sqlite::write_transaction(
                    self.connection,
                    operation.clone(),
                    |tx| {
                        if let Some(existing) =
                            lookup(tx, &message.session, &operation, self.limits)?
                        {
                            return Ok(existing);
                        }
                        let used = capacity(tx, self.limits)?;
                        let session = message.session.clone();
                        let (result, ordinal) =
                            super::append::apply_append(tx, *message, self.limits)?;
                        let SessionResult::Appended {
                            message,
                            deduplicated,
                        } = result
                        else {
                            return Err(invalid_receipt());
                        };
                        let generation = storage_generation(tx, &session.id)?;
                        let receipt = SessionAppendReceipt {
                            operation: operation.clone(),
                            version: SessionVersion::issue(&session.id, &generation, ordinal)?,
                            message_id: message.id,
                            deduplicated,
                            durability: StorageDurability::SqliteWalFull,
                        };
                        let charge = crate::storage::receipt_budget::charge([
                            session.id.as_str(),
                            operation.key.as_str(),
                            operation.request_fingerprint.as_str(),
                            receipt.message_id.as_str(),
                        ])?;
                        crate::storage::receipt_budget::admit(self.limits, used, charge)?;
                        tx.execute("INSERT INTO session_receipts(session_id,operation,expires,fingerprint,kind,version,message_id,deduplicated)
                        VALUES(?1,?2,?3,?4,'append',?5,?6,?7)", params![session.id,operation.key.as_str(),operation.key.expires_at() as i64,
                            operation.request_fingerprint,receipt.version.as_token(),receipt.message_id,receipt.deduplicated]).map_err(sqlite_error)?;
                        if let Some(context) = self.operation {
                            context.check()?;
                        }
                        Ok(SessionWriteReceipt::Append(receipt))
                    },
                );
                Ok(SessionWriteResult::Outcome(outcome))
            }
        }
    }
}

pub(super) fn lookup(
    connection: &Connection,
    session: &SessionRef,
    operation: &StorageOperationRef,
    limits: &crate::StorageLimits,
) -> Result<Option<SessionWriteReceipt>, HostError> {
    super::maintenance::reject_other_operation(connection, session, operation)?;
    let stored = connection.query_row("SELECT fingerprint,version,message_id,deduplicated,kind,generation,created,summary_text,summary_message_count FROM session_receipts WHERE session_id=?1 AND operation=?2",
        params![session.id,operation.key.as_str()], |row| {
            for index in [0,1,2,4,5,7] {
                if let rusqlite::types::ValueRef::Text(text) = row.get_ref(index)?
                    && text.len() > if matches!(index,2|7) { limits.max_value_bytes.min(limits.max_result_bytes) } else { 256 } { return Err(rusqlite::Error::InvalidQuery); }
            }
            Ok(StoredReceipt { fingerprint: row.get(0)?, version: row.get(1)?, message_id: row.get(2)?, deduplicated: row.get(3)?, kind: row.get(4)?, generation: row.get(5)?, created: row.get(6)?, summary_text: row.get(7)?, summary_message_count: row.get(8)? })
        }).optional().map_err(sqlite_error)?;
    let Some(stored) = stored else {
        return Ok(None);
    };
    if stored.fingerprint != operation.request_fingerprint {
        return Err(crate::session::write::mismatch());
    }
    if stored.kind == "context" {
        return Err(invalid_request(
            "context operation requires context reconciliation",
        ));
    }
    if stored.kind == "compact" {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "legacy automatic compaction receipt requires migration to context publication",
        ));
    }
    if stored.summary_text.is_some() || stored.summary_message_count.is_some() {
        return Err(invalid_receipt());
    }
    if stored.kind == "resolve" {
        let (Some(generation), Some(created), None, None, None) = (
            stored.generation,
            stored.created,
            stored.version,
            stored.message_id,
            stored.deduplicated,
        ) else {
            return Err(invalid_receipt());
        };
        if !valid_generation(&generation) || !matches!(created, 0 | 1) {
            return Err(invalid_receipt());
        }
        return Ok(Some(SessionWriteReceipt::Resolve(SessionResolveReceipt {
            operation: operation.clone(),
            session: session.clone(),
            created: created == 1,
            generation: crate::session::SessionGeneration::issue(&session.id, &generation)?,
            durability: StorageDurability::SqliteWalFull,
        })));
    }
    let (Some(version), Some(message_id), Some(deduplicated)) =
        (stored.version, stored.message_id, stored.deduplicated)
    else {
        return Err(invalid_receipt());
    };
    if stored.kind != "append"
        || stored.generation.is_some()
        || stored.created.is_some()
        || message_id.is_empty()
        || !matches!(deduplicated, 0 | 1)
    {
        return Err(invalid_receipt());
    }
    let version = SessionVersion::parse(&version).map_err(|_| invalid_receipt())?;
    if !version.belongs_to(&session.id) {
        return Err(invalid_receipt());
    }
    Ok(Some(SessionWriteReceipt::Append(SessionAppendReceipt {
        operation: operation.clone(),
        version,
        message_id,
        deduplicated: deduplicated == 1,
        durability: StorageDurability::SqliteWalFull,
    })))
}
struct StoredReceipt {
    summary_text: Option<String>,
    summary_message_count: Option<i64>,
    fingerprint: String,
    kind: String,
    version: Option<String>,
    message_id: Option<String>,
    deduplicated: Option<i64>,
    generation: Option<String>,
    created: Option<i64>,
}
fn valid_generation(value: &str) -> bool {
    value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn storage_generation(connection: &Connection, session: &str) -> Result<String, HostError> {
    let generation = connection
        .query_row(
            "SELECT generation FROM session_storage_generations WHERE session_id=?1",
            [session],
            |row| bounded_text(row, 0, 32),
        )
        .map_err(sqlite_error)?;
    if !valid_generation(&generation) {
        return Err(invalid_receipt());
    }
    Ok(generation)
}
pub(super) fn capacity(
    connection: &Connection,
    limits: &crate::StorageLimits,
) -> Result<usize, HostError> {
    connection
        .execute(
            "DELETE FROM session_retention_receipts WHERE expires<=?1",
            [crate::storage::receipt::now()? as i64],
        )
        .map_err(sqlite_error)?;
    connection
        .execute(
            "DELETE FROM session_receipts WHERE expires<=?1",
            [crate::storage::receipt::now()? as i64],
        )
        .map_err(sqlite_error)?;
    let maximum =
        i64::try_from(limits.max_receipts).map_err(|_| crate::session::write::limit_error())?;
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM (SELECT 1 FROM session_receipts UNION ALL SELECT 1 FROM session_retention_receipts LIMIT ?1)",
            [maximum],
            |row| row.get(0),
        )
        .map_err(sqlite_error)?;
    if count >= maximum {
        return Err(crate::session::write::limit_error());
    }
    let used: i64 = connection.query_row("SELECT COALESCE(SUM(?1 + 2 * (length(CAST(session_id AS BLOB)) + length(CAST(operation AS BLOB)) + length(CAST(fingerprint AS BLOB)) + length(CAST(COALESCE(message_id,session_id) AS BLOB)) + COALESCE(length(CAST(summary_text AS BLOB)),0))), 0) FROM session_receipts", [crate::storage::receipt_budget::FIXED_BYTES as i64], |row| row.get(0)).map_err(sqlite_error)?;
    let maintenance: i64 = connection.query_row("SELECT COALESCE(SUM(?1 + 2 * (length(CAST(session_id AS BLOB)) + length(CAST(operation AS BLOB)) + length(CAST(fingerprint AS BLOB)) + length(CAST(generation AS BLOB)))),0) FROM session_retention_receipts", [crate::storage::receipt_budget::FIXED_BYTES as i64], |row| row.get(0)).map_err(sqlite_error)?;
    used.checked_add(maintenance)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(crate::storage::receipt_budget::exceeded)
}
fn invalid_receipt() -> HostError {
    HostError::new(
        HostErrorCode::SchemaMismatch,
        "invalid stored session receipt",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn obsolete_compaction_receipt_cannot_reactivate_removed_protocol() {
        let mut connection = Connection::open_in_memory().unwrap();
        super::super::schema::initialize_schema(&mut connection).unwrap();
        let operation = StorageOperationRef {
            key: crate::StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap(),
            request_fingerprint: "a".repeat(64),
        };
        connection.execute("INSERT INTO session_receipts(session_id,operation,expires,fingerprint,kind,generation,summary_text,summary_message_count) VALUES('session',?1,?2,?3,'compact',?4,'legacy summary',1)", params![operation.key.as_str(), operation.key.expires_at() as i64, operation.request_fingerprint, "0".repeat(32)]).unwrap();
        let error = lookup(
            &connection,
            &SessionRef {
                id: "session".into(),
            },
            &operation,
            &crate::StorageLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, HostErrorCode::SchemaMismatch);
        assert!(error.message.contains("migration to context publication"));
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM session_receipts", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1, "rejection must preserve the original evidence");
    }

    #[test]
    fn resolve_commit_failure_is_unknown_and_not_an_empty_creation_result() {
        let mut connection = Connection::open_in_memory().unwrap();
        super::super::schema::initialize_schema(&mut connection).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE commit_parent(id INTEGER PRIMARY KEY);
            CREATE TABLE commit_child(parent INTEGER REFERENCES commit_parent(id) DEFERRABLE INITIALLY DEFERRED);
            CREATE TRIGGER fail_commit AFTER INSERT ON session_receipts BEGIN INSERT INTO commit_child VALUES(1); END;").unwrap();
        let limits = crate::StorageLimits::default();
        let mut db = SessionDatabase {
            connection: &mut connection,
            operation: None,
            limits: &limits,
        };
        let response = db
            .execute_write(SessionWriteOperation::Resolve {
                key: crate::StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap(),
                config: SessionConfig {
                    id: "unknown-resolve".into(),
                    context: ContextPolicy::All,
                    retention: RetentionPolicy::Forever,
                },
            })
            .unwrap();
        let SessionWriteResult::Outcome(WriteOutcome::Unknown { operation, .. }) = response else {
            panic!("commit must be unknown")
        };
        assert_eq!(
            db.execute_write(SessionWriteOperation::Reconcile {
                session: SessionRef {
                    id: "unknown-resolve".into()
                },
                operation
            })
            .unwrap(),
            SessionWriteResult::Receipt(ReceiptLookup::Unresolved)
        );
    }
    #[test]
    fn failed_commit_remains_unknown_until_authoritative_reconciliation() {
        let mut connection = Connection::open_in_memory().unwrap();
        super::super::schema::initialize_schema(&mut connection).unwrap();
        connection.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE commit_parent(id INTEGER PRIMARY KEY);
            CREATE TABLE commit_child(parent INTEGER REFERENCES commit_parent(id) DEFERRABLE INITIALLY DEFERRED);
            CREATE TRIGGER fail_commit AFTER INSERT ON session_receipts BEGIN INSERT INTO commit_child VALUES(1); END;").unwrap();
        let limits = crate::StorageLimits::default();
        let session = SessionRef {
            id: "commit-error".into(),
        };
        let mut db = SessionDatabase {
            connection: &mut connection,
            operation: None,
            limits: &limits,
        };
        db.resolve(SessionConfig {
            id: session.id.clone(),
            context: ContextPolicy::All,
            retention: RetentionPolicy::Forever,
        })
        .unwrap();
        let response = db
            .execute_write(SessionWriteOperation::Append {
                key: crate::StorageOperationKey::new(std::time::Duration::from_secs(3600)).unwrap(),
                message: Box::new(SessionMessage {
                    id: "message".into(),
                    session: session.clone(),
                    from: None,
                    to: None,
                    role: SessionMessageRole::User,
                    created_at: "0".into(),
                    payload: crate::HostValue::String("value".into()),
                    provenance: None,
                    dedup_key: None,
                }),
            })
            .unwrap();
        let SessionWriteResult::Outcome(WriteOutcome::Unknown { operation, .. }) = response else {
            panic!("commit must remain unknown");
        };
        assert_eq!(
            db.execute_write(SessionWriteOperation::Reconcile { session, operation })
                .unwrap(),
            SessionWriteResult::Receipt(ReceiptLookup::Unresolved)
        );
    }
}
