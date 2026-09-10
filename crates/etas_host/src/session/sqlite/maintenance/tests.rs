use super::*;
use crate::session::{SessionHistoryFence, SessionMaintenanceResponse};

fn setup(connection: &mut Connection, limits: &crate::StorageLimits) -> SessionHistoryFence {
    super::super::schema::initialize_schema(connection).unwrap();
    super::super::append::apply_resolve(
        connection,
        SessionConfig {
            id: "s".into(),
            context: ContextPolicy::All,
            retention: RetentionPolicy::Days(1),
        },
        limits,
    )
    .unwrap();
    for id in ["a", "b"] {
        super::super::append::apply_append(
            connection,
            SessionMessage {
                id: id.into(),
                session: SessionRef { id: "s".into() },
                role: SessionMessageRole::User,
                from: None,
                to: None,
                created_at: "0".into(),
                payload: crate::HostValue::String("private data".into()),
                provenance: None,
                dedup_key: Some(id.into()),
            },
            limits,
        )
        .unwrap();
    }
    let mut db = SessionDatabase {
        connection,
        operation: None,
        limits,
    };
    let SessionResult::History { fence, .. } = db
        .execute_operation(SessionOperation::Load {
            session: SessionRef { id: "s".into() },
            context: ContextPolicy::All,
            cursor: None,
            limit: Some(1),
        })
        .unwrap()
    else {
        panic!("history")
    };
    fence
}
fn prepared(fence: SessionHistoryFence, limits: &crate::StorageLimits) -> SessionRetentionIntent {
    SessionRetentionIntent::prepare(
        SessionRef { id: "s".into() },
        fence,
        -1,
        2,
        crate::StorageOperationKey::new(std::time::Duration::from_secs(60)).unwrap(),
        limits,
    )
    .unwrap()
}
fn counts(connection: &Connection) -> (i64, i64) {
    (
        connection
            .query_row("SELECT COUNT(*) FROM session_messages", [], |row| {
                row.get(0)
            })
            .unwrap(),
        connection
            .query_row(
                "SELECT COUNT(*) FROM session_retention_receipts",
                [],
                |row| row.get(0),
            )
            .unwrap(),
    )
}

#[test]
fn commit_failure_preserves_unknown_and_query_only_reconciliation() {
    let mut connection = Connection::open_in_memory().unwrap();
    let limits = crate::StorageLimits::default();
    let fence = setup(&mut connection, &limits);
    let intent = prepared(fence.clone(), &limits);
    let operation = intent.operation.clone();
    connection.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE commit_parent(id INTEGER PRIMARY KEY);
        CREATE TABLE commit_child(parent INTEGER REFERENCES commit_parent(id) DEFERRABLE INITIALLY DEFERRED);
        CREATE TRIGGER fail_retention_commit AFTER INSERT ON session_retention_receipts BEGIN INSERT INTO commit_child VALUES(1); END;").unwrap();
    let result = execute(
        &mut SessionDatabase {
            connection: &mut connection,
            operation: None,
            limits: &limits,
        },
        SessionMaintenanceOperation::Retain(Box::new(intent)),
    )
    .unwrap();
    assert!(
        matches!(&result,SessionMaintenanceResult::Outcome(WriteOutcome::Unknown{operation:actual,..}) if actual==&operation)
    );
    use crate::execution::OperationResponse;
    let response = SessionMaintenanceResponse {
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
    assert_eq!(counts(&connection), (2, 0));
    assert!(
        fence
            .is_current(
                &super::super::fence::state(&connection, "s").unwrap(),
                &limits
            )
            .unwrap()
    );
    let result = execute(
        &mut SessionDatabase {
            connection: &mut connection,
            operation: None,
            limits: &limits,
        },
        SessionMaintenanceOperation::Reconcile {
            session: SessionRef { id: "s".into() },
            operation,
        },
    )
    .unwrap();
    assert_eq!(
        result,
        SessionMaintenanceResult::Receipt(ReceiptLookup::Unresolved)
    );
    assert_eq!(counts(&connection), (2, 0));
}

#[test]
fn failed_staging_rolls_back_deletion_and_history_revision() {
    let mut connection = Connection::open_in_memory().unwrap();
    let limits = crate::StorageLimits::default();
    let fence = setup(&mut connection, &limits);
    let intent = prepared(fence.clone(), &limits);
    connection.execute_batch("CREATE TRIGGER fail_retention_receipt BEFORE INSERT ON session_retention_receipts BEGIN SELECT RAISE(ABORT,'receipt failure'); END;").unwrap();
    let result = execute(
        &mut SessionDatabase {
            connection: &mut connection,
            operation: None,
            limits: &limits,
        },
        SessionMaintenanceOperation::Retain(Box::new(intent)),
    )
    .unwrap();
    assert!(matches!(
        result,
        SessionMaintenanceResult::Outcome(WriteOutcome::NotCommitted {
            reason: crate::session::SessionRetentionRejection::Rejected(_),
            ..
        })
    ));
    assert_eq!(counts(&connection), (2, 0));
    assert!(
        fence
            .is_current(
                &super::super::fence::state(&connection, "s").unwrap(),
                &limits
            )
            .unwrap()
    );
}
