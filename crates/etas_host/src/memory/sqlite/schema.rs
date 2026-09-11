use super::sqlite_error;
use crate::storage::version::{StoreGeneration, next_revision, scope_identity};
use crate::{HostError, HostErrorCode, MemoryVersion, StoreRef};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

pub(super) struct StoreState {
    pub scope: String,
    pub generation: StoreGeneration,
    pub revision: i64,
}

impl StoreState {
    pub fn version(&self, revision: i64) -> Result<MemoryVersion, HostError> {
        if revision <= 0 || revision > self.revision {
            return Err(HostError::new(
                HostErrorCode::SchemaMismatch,
                "entry revision is outside committed Store history",
            ));
        }
        MemoryVersion::issue(&self.scope, &self.generation, revision)
    }
}

pub(super) fn initialize(connection: &mut Connection) -> Result<(), HostError> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sqlite_error)?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS etas_memory_schema (
        id INTEGER PRIMARY KEY CHECK(id = 1), format INTEGER NOT NULL, namespace TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS memory_entries (
        region TEXT NOT NULL, path TEXT NOT NULL, key_json TEXT NOT NULL,
        value_json TEXT NOT NULL, version INTEGER NOT NULL,
        PRIMARY KEY(region, path, key_json));",
    )
    .map_err(sqlite_error)?;
    let format: Option<i64> = tx
        .query_row(
            "SELECT format FROM etas_memory_schema WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(sqlite_error)?;
    match format {
        Some(1..=3) => {}
        Some(_) => {
            return Err(HostError::new(
                HostErrorCode::SchemaMismatch,
                "unsupported memory database format",
            ));
        }
        None => {
            tx.execute_batch(
                "CREATE TABLE memory_stores (
                region TEXT NOT NULL, path TEXT NOT NULL, generation TEXT NOT NULL,
                revision INTEGER NOT NULL CHECK(revision >= 0), PRIMARY KEY(region, path));
                CREATE TEMP TABLE memory_migration AS SELECT region, path, key_json, value_json,
                    ROW_NUMBER() OVER (PARTITION BY region, path ORDER BY key_json) AS version
                    FROM memory_entries;
                DELETE FROM memory_entries;
                INSERT INTO memory_entries SELECT * FROM memory_migration;
                DROP TABLE memory_migration;",
            )
            .map_err(sqlite_error)?;
            let mut statement = tx
                .prepare("SELECT region,path,MAX(version) FROM memory_entries GROUP BY region,path")
                .map_err(sqlite_error)?;
            let mut rows = statement.query([]).map_err(sqlite_error)?;
            while let Some(row) = rows.next().map_err(sqlite_error)? {
                let region: String = row.get(0).map_err(sqlite_error)?;
                let path: String = row.get(1).map_err(sqlite_error)?;
                let revision: i64 = row.get(2).map_err(sqlite_error)?;
                tx.execute(
                    "INSERT INTO memory_stores VALUES (?1,?2,?3,?4)",
                    params![region, path, StoreGeneration::new()?.as_str(), revision],
                )
                .map_err(sqlite_error)?;
            }
            tx.execute(
                "INSERT INTO etas_memory_schema VALUES (1,1,?1)",
                [StoreGeneration::new()?.as_str()],
            )
            .map_err(sqlite_error)?;
        }
    }
    let invalid: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM memory_entries e
        LEFT JOIN memory_stores s ON e.region=s.region AND e.path=s.path
        WHERE s.generation IS NULL OR e.version <= 0 OR e.version > s.revision)",
            [],
            |row| row.get(0),
        )
        .map_err(sqlite_error)?;
    if invalid {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "invalid memory Store revision metadata",
        ));
    }
    tx.execute("CREATE UNIQUE INDEX IF NOT EXISTS memory_store_revision ON memory_entries(region,path,version)", []).map_err(sqlite_error)?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS memory_receipts (
        region TEXT NOT NULL, path TEXT NOT NULL, operation TEXT NOT NULL, expires INTEGER NOT NULL,
        fingerprint TEXT NOT NULL, kind TEXT NOT NULL, version TEXT, expected TEXT, actual TEXT,
        PRIMARY KEY(region,path,operation));
        CREATE INDEX IF NOT EXISTS memory_receipt_expiry ON memory_receipts(expires);",
    )
    .map_err(sqlite_error)?;
    if format != Some(3) {
        // Old receipts remain reserved. Missing target evidence is rejected on
        // replay, never treated as permission to execute the old write again.
        tx.execute_batch(
            "ALTER TABLE memory_receipts ADD COLUMN key_json TEXT;
            ALTER TABLE memory_receipts ADD COLUMN schema_fingerprint TEXT;
            UPDATE etas_memory_schema SET format=3 WHERE id=1;",
        )
        .map_err(sqlite_error)?;
    }
    tx.commit().map_err(sqlite_error)
}

pub(super) fn state(
    connection: &Connection,
    store: &StoreRef,
    path: &str,
) -> Result<Option<StoreState>, HostError> {
    let namespace: String = connection
        .query_row(
            "SELECT namespace FROM etas_memory_schema WHERE id=1",
            [],
            |row| row.get(0),
        )
        .map_err(sqlite_error)?;
    let namespace = StoreGeneration::from_stored(namespace)?;
    connection
        .query_row(
            "SELECT generation,revision FROM memory_stores WHERE region=?1 AND path=?2",
            params![store.region.stable_id, path],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(sqlite_error)?
        .map(|(generation, revision)| {
            if revision < 0 {
                return Err(HostError::new(
                    HostErrorCode::SchemaMismatch,
                    "negative Store revision",
                ));
            }
            Ok(StoreState {
                scope: scope_identity(namespace.as_str(), &store.region.stable_id, &store.path),
                generation: StoreGeneration::from_stored(generation)?,
                revision,
            })
        })
        .transpose()
}

pub(super) fn ensure_store(
    connection: &Connection,
    store: &StoreRef,
    path: &str,
) -> Result<StoreState, HostError> {
    if let Some(state) = state(connection, store, path)? {
        return Ok(state);
    }
    connection
        .execute(
            "INSERT INTO memory_stores VALUES (?1,?2,?3,0)",
            params![
                store.region.stable_id,
                path,
                StoreGeneration::new()?.as_str()
            ],
        )
        .map_err(sqlite_error)?;
    state(connection, store, path)?
        .ok_or_else(|| HostError::new(HostErrorCode::SchemaMismatch, "missing newly created Store"))
}

pub(super) fn allocate(
    connection: &Connection,
    store: &StoreRef,
    path: &str,
    state: &mut StoreState,
) -> Result<MemoryVersion, HostError> {
    let next = next_revision(state.revision)?;
    let changed = connection
        .execute(
            "UPDATE memory_stores SET revision=?3 WHERE region=?1 AND path=?2 AND revision=?4",
            params![store.region.stable_id, path, next, state.revision],
        )
        .map_err(sqlite_error)?;
    if changed != 1 {
        return Err(HostError::new(
            HostErrorCode::ProviderUnavailable,
            "Store revision allocation lost transaction ownership",
        ));
    }
    state.revision = next;
    state.version(next)
}
