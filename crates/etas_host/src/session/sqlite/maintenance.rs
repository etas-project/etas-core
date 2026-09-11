use super::*;
use crate::session::maintenance::{cutoff, expired, rejected};
use crate::session::{
    RetentionProgress, SessionGeneration, SessionMaintenanceOperation, SessionMaintenanceResult,
    SessionRetentionIntent, SessionRetentionReceipt,
};
use crate::{ReceiptLookup, StorageDurability, StorageOperationRef, WriteOutcome};

mod client;

pub(super) fn execute(
    db: &mut SessionDatabase<'_>,
    request: SessionMaintenanceOperation,
) -> Result<SessionMaintenanceResult, HostError> {
    match request {
        SessionMaintenanceOperation::Reconcile { session, operation } => {
            operation.validate()?;
            Ok(SessionMaintenanceResult::Receipt(
                if operation.key.is_expired()? {
                    ReceiptLookup::Expired
                } else {
                    match lookup(db.connection, &session, &operation)? {
                        Some(receipt) => {
                            receipt.check_result_size(db.limits)?;
                            ReceiptLookup::Found(receipt)
                        }
                        None => ReceiptLookup::Unresolved,
                    }
                },
            ))
        }
        SessionMaintenanceOperation::Retain(intent) => {
            intent.validate(db.limits)?;
            let operation = intent.operation.clone();
            let outcome =
                crate::storage::sqlite::write_transaction(db.connection, operation, |tx| {
                    retain(tx, *intent, db.limits, db.operation)
                });
            Ok(SessionMaintenanceResult::Outcome(match outcome {
                WriteOutcome::Committed(receipt) => receipt.into_outcome(),
                WriteOutcome::NotCommitted { operation, reason } => rejected(operation, reason),
                WriteOutcome::Unknown { operation, error } => {
                    WriteOutcome::Unknown { operation, error }
                }
            }))
        }
    }
}

fn retain(
    tx: &Connection,
    intent: SessionRetentionIntent,
    limits: &crate::StorageLimits,
    operation: Option<&SqliteOperation>,
) -> Result<SessionRetentionReceipt, HostError> {
    if let Some(receipt) = lookup(tx, &intent.session, &intent.operation)? {
        receipt.check_result_size(limits)?;
        return Ok(receipt);
    }
    let used = super::receipt::capacity(tx, limits)?;
    let before = super::fence::state(tx, &intent.session.id)?;
    let selection = intent.fence.retention_selection(&before, limits)?;
    let config = tx
        .query_row(
            "SELECT config_json FROM sessions WHERE id=?1",
            [&intent.session.id],
            |row| bounded_text(row, 0, limits.max_value_bytes),
        )
        .map_err(sqlite_error)?;
    let config: Value = serde_json::from_str(&config).map_err(json_error)?;
    let retention = retention_policy_from_json(
        config
            .get("retention")
            .ok_or_else(|| invalid_request("missing retention policy"))?,
    )?;
    let cutoff = cutoff(selection.retained_after, &retention)?;
    let mut progress = RetentionProgress {
        scanned: 0,
        deleted_messages: 0,
        deleted_dedup_keys: 0,
        next_after: None,
    };
    let mut deletes = Vec::new();
    let mut after = intent.after_ordinal;
    let mut bytes = 0usize;
    {
        let mut statement = tx
            .prepare(
                "SELECT ordinal,created_at,dedup_key IS NOT NULL FROM session_messages
            WHERE session_id=?1 AND ordinal>?2 AND ordinal<=?3 ORDER BY ordinal LIMIT ?4",
            )
            .map_err(sqlite_error)?;
        let mut rows = statement
            .query(params![
                intent.session.id,
                intent.after_ordinal,
                selection.upper,
                intent.scan_limit
            ])
            .map_err(sqlite_error)?;
        while let Some(row) = rows.next().map_err(sqlite_error)? {
            if let Some(operation) = operation {
                operation.check()?;
            }
            after = row.get(0).map_err(sqlite_error)?;
            let timestamp = bounded_text(row, 1, limits.max_value_bytes).map_err(sqlite_error)?;
            bytes = bytes
                .checked_add(timestamp.len())
                .and_then(|n| n.checked_add(16))
                .filter(|n| *n <= limits.max_result_bytes)
                .ok_or_else(crate::session::write::limit_error)?;
            progress.scanned += 1;
            if expired(&timestamp, cutoff)? {
                progress.deleted_messages += 1;
                progress.deleted_dedup_keys +=
                    u32::from(row.get::<_, bool>(2).map_err(sqlite_error)?);
                deletes.push(after);
            }
        }
    }
    if progress.scanned == intent.scan_limit && after < selection.upper {
        progress.next_after = Some(after);
    }
    for ordinal in deletes {
        if let Some(operation) = operation {
            operation.check()?;
        }
        if tx
            .execute(
                "DELETE FROM session_messages WHERE session_id=?1 AND ordinal=?2",
                params![intent.session.id, ordinal],
            )
            .map_err(sqlite_error)?
            != 1
        {
            return Err(invalid_request(
                "retention selection changed within transaction",
            ));
        }
    }
    let state = super::fence::state(tx, &intent.session.id)?;
    let receipt = SessionRetentionReceipt {
        operation: intent.operation.clone(),
        session: intent.session.clone(),
        generation: SessionGeneration::issue(&intent.session.id, &state.revision)?,
        progress,
        durability: StorageDurability::SqliteWalFull,
    };
    receipt.check_result_size(limits)?;
    crate::storage::receipt_budget::admit(limits, used, receipt.charge()?)?;
    tx.execute("INSERT INTO session_retention_receipts(session_id,operation,expires,fingerprint,generation,scanned,deleted,dedup_removed,next_after)
        VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![intent.session.id,intent.operation.key.as_str(),intent.operation.key.expires_at() as i64,
        intent.operation.request_fingerprint,receipt.generation.as_token(),receipt.progress.scanned,receipt.progress.deleted_messages,
        receipt.progress.deleted_dedup_keys,receipt.progress.next_after]).map_err(sqlite_error)?;
    if let Some(operation) = operation {
        operation.check()?;
    }
    Ok(receipt)
}

fn lookup(
    connection: &Connection,
    session: &SessionRef,
    operation: &StorageOperationRef,
) -> Result<Option<SessionRetentionReceipt>, HostError> {
    if connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM session_receipts WHERE session_id=?1 AND operation=?2)",
            params![session.id, operation.key.as_str()],
            |row| row.get::<_, bool>(0),
        )
        .map_err(sqlite_error)?
    {
        return Err(crate::session::write::mismatch());
    }
    let row = connection.query_row("SELECT fingerprint,generation,scanned,deleted,dedup_removed,next_after FROM session_retention_receipts WHERE session_id=?1 AND operation=?2",
        params![session.id,operation.key.as_str()], |row| Ok((bounded_text(row,0,64)?,bounded_text(row,1,128)?,
            row.get::<_,u32>(2)?,row.get::<_,u32>(3)?,row.get::<_,u32>(4)?,row.get::<_,Option<i64>>(5)?))).optional().map_err(sqlite_error)?;
    let Some((fingerprint, generation, scanned, deleted_messages, deleted_dedup_keys, next_after)) =
        row
    else {
        return Ok(None);
    };
    if fingerprint != operation.request_fingerprint {
        return Err(crate::session::write::mismatch());
    }
    let generation = SessionGeneration::parse(&generation)?;
    if !generation.belongs_to(&session.id)
        || deleted_dedup_keys > deleted_messages
        || deleted_messages > scanned
        || next_after.is_some_and(|n| n < 0)
    {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "invalid retention receipt",
        ));
    }
    Ok(Some(SessionRetentionReceipt {
        operation: operation.clone(),
        session: session.clone(),
        generation,
        progress: RetentionProgress {
            scanned,
            deleted_messages,
            deleted_dedup_keys,
            next_after,
        },
        durability: StorageDurability::SqliteWalFull,
    }))
}

pub(super) fn reject_other_operation(
    connection: &Connection,
    session: &SessionRef,
    operation: &StorageOperationRef,
) -> Result<(), HostError> {
    if connection.query_row("SELECT EXISTS(SELECT 1 FROM session_retention_receipts WHERE session_id=?1 AND operation=?2)",params![session.id,operation.key.as_str()],|row|row.get::<_,bool>(0)).map_err(sqlite_error)? {
        return Err(crate::session::write::mismatch());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
