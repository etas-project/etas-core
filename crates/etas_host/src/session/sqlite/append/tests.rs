use super::*;

fn config(id: &str) -> SessionConfig {
    SessionConfig {
        id: id.into(),
        context: ContextPolicy::All,
        retention: RetentionPolicy::Forever,
    }
}

#[test]
fn resolve_checks_stored_config_size_before_decoding_or_comparing() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_schema(&mut connection).unwrap();
    let limits = crate::StorageLimits {
        max_value_bytes: 256,
        ..Default::default()
    };
    let mut database = SessionDatabase {
        connection: &mut connection,
        operation: None,
        limits: &limits,
    };
    database.resolve(config("bounded")).unwrap();
    database
        .connection
        .execute("UPDATE sessions SET config_json=?1", [" ".repeat(4096)])
        .unwrap();
    let error = database.resolve(config("bounded")).unwrap_err();
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    assert_eq!(
        database
            .connection
            .query_row("SELECT length(config_json) FROM sessions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        4096
    );
}

#[test]
fn resolve_preflights_escaped_configuration_size_without_creating_session() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_schema(&mut connection).unwrap();
    let limits = crate::StorageLimits {
        max_value_bytes: 256,
        ..Default::default()
    };
    let mut database = SessionDatabase {
        connection: &mut connection,
        operation: None,
        limits: &limits,
    };
    let error = database.resolve(config(&"\\".repeat(128))).unwrap_err();
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    assert_eq!(
        database
            .connection
            .query_row("SELECT count(*) FROM sessions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn resolve_staging_and_identity_allocation_roll_back_together() {
    let mut connection = Connection::open_in_memory().unwrap();
    initialize_schema(&mut connection).unwrap();
    let limits = crate::StorageLimits::default();
    let transaction = connection.transaction().unwrap();
    assert!(matches!(
        apply_resolve(&transaction, config("atomic"), &limits).unwrap(),
        SessionResult::Resolved { created: true, .. }
    ));
    transaction.rollback().unwrap();
    for table in [
        "sessions",
        "session_sequences",
        "session_storage_generations",
        "session_history_generations",
    ] {
        assert_eq!(
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
