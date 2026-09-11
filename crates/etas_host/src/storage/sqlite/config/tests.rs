use super::*;
use crate::TestWorkspace;

#[test]
fn durable_settings_are_verified_and_worker_wait_is_bounded() {
    let workspace = TestWorkspace::create("sqlite-shared-config").unwrap();
    let connection = open_durable(
        &workspace.path().join("nested/storage.db"),
        &StorageLimits::default(),
        |connection| {
            verify_durability(connection)?;
            let wait: i64 = connection
                .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                .unwrap();
            assert_eq!(wait, 5000);
            connection
                .execute_batch("CREATE TABLE evidence(value INTEGER)")
                .unwrap();
            Ok(())
        },
    )
    .unwrap();
    configure_worker(&connection, &StorageLimits::default()).unwrap();
    verify_durability(&connection).unwrap();
    let wait: i64 = connection
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();
    assert_eq!(wait, 100);
    assert_eq!(
        connection.limit(Limit::SQLITE_LIMIT_LENGTH),
        14 * 1024 * 1024
    );
}

#[test]
fn migration_cannot_weaken_durability() {
    let workspace = TestWorkspace::create("sqlite-weakened-config").unwrap();
    let error = open_durable(
        &workspace.path().join("storage.db"),
        &StorageLimits::default(),
        |connection| {
            connection
                .pragma_update(None, "synchronous", "OFF")
                .map_err(config_error)
        },
    )
    .unwrap_err();
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
    assert!(error.message.contains("synchronous=FULL"));
}

#[test]
fn durable_adapter_rejects_connection_without_wal() {
    let mut migrated = false;
    let error = open_durable(Path::new(":memory:"), &StorageLimits::default(), |_| {
        migrated = true;
        Ok(())
    })
    .unwrap_err();
    assert!(!migrated);
    assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
    assert!(error.message.contains("WAL"));
}

#[test]
fn invalid_limits_are_rejected_before_creating_storage() {
    let workspace = TestWorkspace::create("sqlite-invalid-config").unwrap();
    let path = workspace.path().join("not-created/storage.db");
    let error = open_durable(
        &path,
        &StorageLimits {
            max_value_bytes: 0,
            ..Default::default()
        },
        |_| panic!("invalid configuration must not run migrations"),
    )
    .unwrap_err();
    assert_eq!(error.code, HostErrorCode::InvalidRequest);
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn engine_rejects_preexisting_oversized_blob_before_owned_decode() {
    let workspace = TestWorkspace::create("sqlite-row-limit").unwrap();
    let path = workspace.path().join("storage.db");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE evidence(value BLOB); INSERT INTO evidence VALUES(zeroblob(65536));",
            )
            .unwrap();
    }
    let limits = StorageLimits {
        max_pending_bytes: 16384,
        max_value_bytes: 1024,
        max_result_bytes: 4096,
        ..Default::default()
    };
    let connection = open_durable(&path, &limits, |_| Ok(())).unwrap();
    let mut decoded = false;
    let error = connection
        .query_row("SELECT value FROM evidence", [], |row| {
            decoded = true;
            row.get::<_, Vec<u8>>(0)
        })
        .unwrap_err();
    assert!(!decoded);
    assert!(
        matches!(error, rusqlite::Error::SqliteFailure(ref code, _) if code.code == rusqlite::ErrorCode::TooBig)
    );
}

#[test]
fn worker_keeps_stricter_native_row_limit() {
    let connection = Connection::open_in_memory().unwrap();
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 8192);
    configure_worker(&connection, &StorageLimits::default()).unwrap();
    assert_eq!(connection.limit(Limit::SQLITE_LIMIT_LENGTH), 8192);
}

#[test]
fn scratch_budget_overflow_fails_without_raising_native_limit() {
    let connection = Connection::open_in_memory().unwrap();
    let previous = connection.limit(Limit::SQLITE_LIMIT_LENGTH);
    let limits = StorageLimits {
        max_pending_bytes: usize::MAX,
        max_value_bytes: usize::MAX / 2,
        max_result_bytes: 1,
        ..Default::default()
    };
    limits.validate().unwrap();
    let error = configure_worker(&connection, &limits).unwrap_err();
    assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    assert_eq!(connection.limit(Limit::SQLITE_LIMIT_LENGTH), previous);
}
