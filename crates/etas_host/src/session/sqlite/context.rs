use super::*;
use crate::session::{
    SessionContextContent, SessionContextEvidence, SessionContextPublication,
    SessionContextReceipt, SessionContextRejection, SessionPublishedContext, SessionWriteResult,
};
use crate::{ReceiptLookup, StorageOperationRef, WriteOutcome};

pub(super) fn publish(
    db: &mut SessionDatabase<'_>,
    publication: SessionContextPublication,
) -> Result<SessionWriteResult, HostError> {
    publication.validate(db.limits)?;
    let operation = publication.operation.clone();
    if let Err(error) = operation
        .key
        .validate_window(db.limits.max_receipt_retention_seconds)
    {
        return Ok(SessionWriteResult::Context(
            crate::session::context::rejected(operation, error),
        ));
    }
    let result = crate::storage::sqlite::write_transaction(
        db.connection,
        operation.clone(),
        |tx| {
            if let Some(evidence) = lookup(tx, &publication.session, &operation)? {
                return Ok(evidence);
            }
            let used = super::receipt::capacity(tx, db.limits)?;
            let state = super::fence::state(tx, &publication.session.id)?;
            let evidence = if !publication.fence.is_current(&state, db.limits)? {
                SessionContextEvidence::NotCommitted {
                    operation: operation.clone(),
                    reason: SessionContextRejection::StaleHistory,
                }
            } else {
                let next = state
                    .context_version
                    .checked_add(1)
                    .filter(|v| *v <= i64::MAX as u64)
                    .ok_or_else(|| invalid_request("session context version overflow"))?;
                let content = publication.content.encode(db.limits)?;
                tx.execute("INSERT INTO session_contexts(session_id,content,fence,version) VALUES(?1,?2,?3,?4)
                ON CONFLICT(session_id) DO UPDATE SET content=excluded.content,fence=excluded.fence,version=excluded.version",
                params![publication.session.id,content,publication.fence.as_token(),next as i64]).map_err(sqlite_error)?;
                let after = super::fence::state(tx, &publication.session.id)?;
                if after.context_version != next {
                    return Err(crate::session::context::invalid());
                }
                SessionContextEvidence::Committed(SessionContextReceipt {
                    operation: operation.clone(),
                    session: publication.session.clone(),
                    generation: crate::session::SessionGeneration::issue(
                        &publication.session.id,
                        &after.revision,
                    )?,
                    context_version: next,
                    durability: crate::StorageDurability::SqliteWalFull,
                })
            };
            crate::storage::receipt_budget::admit(
                db.limits,
                used,
                evidence.charge(&publication.session.id)?,
            )?;
            let (status, generation, version) = match &evidence {
                SessionContextEvidence::Committed(r) => (
                    "committed",
                    Some(r.generation.as_token()),
                    Some(r.context_version as i64),
                ),
                SessionContextEvidence::NotCommitted { .. } => ("stale", None, None),
            };
            tx.execute("INSERT INTO session_receipts(session_id,operation,expires,fingerprint,kind,generation,context_status,context_version)
            VALUES(?1,?2,?3,?4,'context',?5,?6,?7)", params![publication.session.id,operation.key.as_str(),
            operation.key.expires_at() as i64,operation.request_fingerprint,generation,status,version]).map_err(sqlite_error)?;
            if let Some(operation) = db.operation {
                operation.check()?;
            }
            Ok(evidence)
        },
    );
    Ok(SessionWriteResult::Context(match result {
        WriteOutcome::Committed(evidence) => evidence.into_outcome(),
        WriteOutcome::NotCommitted { operation, reason } => {
            crate::session::context::rejected(operation, reason)
        }
        WriteOutcome::Unknown { operation, error } => WriteOutcome::Unknown { operation, error },
    }))
}

pub(super) fn reconcile(
    connection: &Connection,
    session: &SessionRef,
    operation: &StorageOperationRef,
) -> Result<SessionWriteResult, HostError> {
    operation.validate()?;
    Ok(SessionWriteResult::ContextReceipt(
        if operation.key.is_expired()? {
            ReceiptLookup::Expired
        } else {
            match lookup(connection, session, operation)? {
                Some(r) => ReceiptLookup::Found(r),
                None => ReceiptLookup::Unresolved,
            }
        },
    ))
}

fn lookup(
    connection: &Connection,
    session: &SessionRef,
    operation: &StorageOperationRef,
) -> Result<Option<SessionContextEvidence>, HostError> {
    super::maintenance::reject_other_operation(connection, session, operation)?;
    let stored = connection.query_row("SELECT fingerprint,kind,context_status,generation,context_version FROM session_receipts WHERE session_id=?1 AND operation=?2",
        params![session.id,operation.key.as_str()], |row| {
            let fingerprint = bounded_text(row,0,64)?;
            let kind = bounded_text(row,1,16)?;
            let status = match row.get_ref(2)? {rusqlite::types::ValueRef::Null=>None,_=>Some(bounded_text(row,2,16)?)};
            let generation = match row.get_ref(3)? {rusqlite::types::ValueRef::Null=>None,_=>Some(bounded_text(row,3,128)?)};
            Ok((fingerprint,kind,status,generation,row.get::<_,Option<i64>>(4)?))
        }).optional().map_err(sqlite_error)?;
    let Some((fingerprint, kind, status, generation, version)) = stored else {
        return Ok(None);
    };
    if fingerprint != operation.request_fingerprint || kind != "context" {
        return Err(crate::session::write::mismatch());
    }
    let invalid = crate::session::context::invalid;
    Ok(Some(match (status.as_deref(), generation, version) {
        (Some("committed"), Some(generation), Some(version)) if version > 0 => {
            let generation =
                crate::session::SessionGeneration::parse(&generation).map_err(|_| invalid())?;
            if !generation.belongs_to(&session.id) {
                return Err(invalid());
            }
            SessionContextEvidence::Committed(SessionContextReceipt {
                operation: operation.clone(),
                session: session.clone(),
                generation,
                context_version: version as u64,
                durability: crate::StorageDurability::SqliteWalFull,
            })
        }
        (Some("stale"), None, None) => SessionContextEvidence::NotCommitted {
            operation: operation.clone(),
            reason: SessionContextRejection::StaleHistory,
        },
        _ => return Err(invalid()),
    }))
}

pub(super) fn load(
    connection: &Connection,
    session: &str,
    version: u64,
    limits: &crate::StorageLimits,
) -> Result<Option<SessionPublishedContext>, HostError> {
    let row = connection
        .query_row(
            "SELECT content,fence,version FROM session_contexts WHERE session_id=?1",
            [session],
            |row| {
                Ok((
                    bounded_text(row, 0, limits.max_value_bytes.min(limits.max_result_bytes))?,
                    bounded_text(row, 1, limits.max_value_bytes.min(limits.max_result_bytes))?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(sqlite_error)?;
    row.map(|(content, fence, current)| {
        if current < 1 || current as u64 != version {
            return Err(crate::session::context::invalid());
        }
        let fence = crate::session::SessionHistoryFence::from_token(fence, limits)?;
        if fence.session_id() != session {
            return Err(crate::session::context::invalid());
        }
        Ok(SessionPublishedContext {
            content: SessionContextContent::decode(&content, limits)?,
            fence,
            version,
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_context_commit_preserves_unknown_evidence_and_rolls_back_content() {
        let mut connection = Connection::open_in_memory().unwrap();
        super::super::schema::initialize_schema(&mut connection).unwrap();
        let limits = crate::StorageLimits::default();
        let session = SessionRef {
            id: "commit-failure".into(),
        };
        super::super::append::apply_resolve(
            &connection,
            SessionConfig {
                id: session.id.clone(),
                context: ContextPolicy::All,
                retention: RetentionPolicy::Forever,
            },
            &limits,
        )
        .unwrap();
        let mut db = SessionDatabase {
            connection: &mut connection,
            operation: None,
            limits: &limits,
        };
        let SessionResult::History { fence, .. } = db
            .execute_operation(SessionOperation::Load {
                session: session.clone(),
                context: ContextPolicy::All,
                cursor: None,
                limit: Some(1),
            })
            .unwrap()
        else {
            panic!("history")
        };
        let publication = SessionContextPublication::prepare(
            session.clone(),
            fence,
            SessionContextContent {
                text: "caller content".into(),
                provenance: Default::default(),
            },
            &limits,
        )
        .unwrap();
        db.connection.execute_batch("PRAGMA foreign_keys=ON;
            CREATE TABLE commit_parent(id INTEGER PRIMARY KEY);
            CREATE TABLE commit_child(parent INTEGER REFERENCES commit_parent(id) DEFERRABLE INITIALLY DEFERRED);
            CREATE TRIGGER fail_context_commit AFTER INSERT ON session_receipts WHEN NEW.kind='context'
            BEGIN INSERT INTO commit_child VALUES(1); END;").unwrap();
        let operation = publication.operation.clone();
        let result = publish(&mut db, publication).unwrap();
        assert!(
            matches!(&result, SessionWriteResult::Context(WriteOutcome::Unknown { operation: actual, .. }) if actual == &operation)
        );
        use crate::execution::OperationResponse;
        let response = crate::session::SessionWriteResponse {
            id: crate::HostRequestId(1),
            result: Ok(result),
        };
        assert!(matches!(
            response.external_outcome(),
            crate::execution::ExternalOutcome::StorageWrite(crate::StorageWriteEvidence {
                status: crate::CommitStatus::Unknown,
                ..
            })
        ));
        assert_eq!(
            reconcile(db.connection, &session, &operation).unwrap(),
            SessionWriteResult::ContextReceipt(ReceiptLookup::Unresolved)
        );
        assert!(
            load(db.connection, &session.id, 0, &limits)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            super::super::fence::state(db.connection, &session.id)
                .unwrap()
                .context_version,
            0
        );
    }
}
