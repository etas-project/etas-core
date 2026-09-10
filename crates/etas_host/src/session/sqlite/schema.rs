use super::*;
mod config;
mod context;
pub(super) fn initialize_schema(connection: &mut Connection) -> Result<(), HostError> {
    let transaction = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(sqlite_error)?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS etas_session_schema(id INTEGER PRIMARY KEY CHECK(id=1),format INTEGER NOT NULL)").map_err(sqlite_error)?;
    let version: Option<i64> = transaction
        .query_row(
            "SELECT format FROM etas_session_schema WHERE id=1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(sqlite_error)?;
    if version.is_some_and(|version| !(1..=9).contains(&version)) {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "unsupported session schema version",
        ));
    }
    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                config_json TEXT NOT NULL,
                summary_text TEXT,
                summary_message_count INTEGER
            );
            CREATE TABLE IF NOT EXISTS session_messages (
                session_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                role TEXT NOT NULL,
                from_participant TEXT,
                to_participant TEXT,
                created_at TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                provenance_json TEXT,
                dedup_key TEXT,
                PRIMARY KEY(session_id, message_id),
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_session_messages_dedup
                ON session_messages(session_id, dedup_key)
                WHERE dedup_key IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_session_messages_order
                ON session_messages(session_id, ordinal);",
        )
        .map_err(sqlite_error)?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS session_sequences(session_id TEXT PRIMARY KEY,last_ordinal INTEGER NOT NULL CHECK(last_ordinal>=-1));
    INSERT OR IGNORE INTO session_sequences SELECT sessions.id,COALESCE(MAX(session_messages.ordinal),-1) FROM sessions LEFT JOIN session_messages ON session_messages.session_id=sessions.id GROUP BY sessions.id;
    INSERT OR IGNORE INTO etas_session_schema VALUES(1,1);").map_err(sqlite_error)?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS session_history_generations (
        session_id TEXT PRIMARY KEY, generation TEXT NOT NULL);
        INSERT OR IGNORE INTO session_history_generations SELECT id, lower(hex(randomblob(16))) FROM sessions;
        CREATE TRIGGER IF NOT EXISTS session_history_create AFTER INSERT ON sessions BEGIN
            INSERT INTO session_history_generations VALUES(NEW.id, lower(hex(randomblob(16)))); END;
        CREATE TRIGGER IF NOT EXISTS session_history_remove AFTER DELETE ON sessions BEGIN
            DELETE FROM session_history_generations WHERE session_id=OLD.id; END;
        CREATE TRIGGER IF NOT EXISTS session_history_configuration AFTER UPDATE ON sessions BEGIN
            UPDATE session_history_generations SET generation=lower(hex(randomblob(16))) WHERE session_id=NEW.id; END;
        CREATE TRIGGER IF NOT EXISTS session_history_delete AFTER DELETE ON session_messages BEGIN
            UPDATE session_history_generations SET generation=lower(hex(randomblob(16))) WHERE session_id=OLD.session_id; END;
        CREATE TRIGGER IF NOT EXISTS session_history_update AFTER UPDATE ON session_messages BEGIN
            UPDATE session_history_generations SET generation=lower(hex(randomblob(16))) WHERE session_id IN (OLD.session_id,NEW.session_id); END;
        UPDATE etas_session_schema SET format=2 WHERE id=1;").map_err(sqlite_error)?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS session_storage_generations(session_id TEXT PRIMARY KEY,generation TEXT NOT NULL);
        INSERT OR IGNORE INTO session_storage_generations SELECT id,lower(hex(randomblob(16))) FROM sessions;
        CREATE TRIGGER IF NOT EXISTS session_storage_create AFTER INSERT ON sessions BEGIN
            INSERT INTO session_storage_generations VALUES(NEW.id,lower(hex(randomblob(16)))); END;
        CREATE TRIGGER IF NOT EXISTS session_storage_remove AFTER DELETE ON sessions BEGIN
            DELETE FROM session_storage_generations WHERE session_id=OLD.id; END;
        CREATE TABLE IF NOT EXISTS session_receipts (
            session_id TEXT NOT NULL, operation TEXT NOT NULL, expires INTEGER NOT NULL,
            fingerprint TEXT NOT NULL, version TEXT NOT NULL, message_id TEXT NOT NULL,
            deduplicated INTEGER NOT NULL CHECK(deduplicated IN (0,1)), PRIMARY KEY(session_id,operation));
        CREATE INDEX IF NOT EXISTS session_receipts_expiry ON session_receipts(expires);
        UPDATE etas_session_schema SET format=3 WHERE id=1;").map_err(sqlite_error)?;
    if version.is_none_or(|version| version < 4) {
        transaction.execute_batch("ALTER TABLE session_receipts RENAME TO session_receipts_v3;
            DROP INDEX session_receipts_expiry;
            CREATE TABLE session_receipts (
                session_id TEXT NOT NULL, operation TEXT NOT NULL, expires INTEGER NOT NULL,
                fingerprint TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('append','resolve')),
                version TEXT, message_id TEXT, deduplicated INTEGER,
                generation TEXT, created INTEGER,
                PRIMARY KEY(session_id,operation),
                CHECK((kind='append' AND version IS NOT NULL AND message_id IS NOT NULL AND deduplicated IN (0,1) AND generation IS NULL AND created IS NULL)
                    OR (kind='resolve' AND version IS NULL AND message_id IS NULL AND deduplicated IS NULL AND generation IS NOT NULL AND created IN (0,1))));
            INSERT INTO session_receipts(session_id,operation,expires,fingerprint,kind,version,message_id,deduplicated)
                SELECT session_id,operation,expires,fingerprint,'append',version,message_id,deduplicated FROM session_receipts_v3;
            DROP TABLE session_receipts_v3;
            CREATE INDEX session_receipts_expiry ON session_receipts(expires);").map_err(sqlite_error)?;
    }
    if version.is_none_or(|version| version < 5) {
        transaction.execute_batch("ALTER TABLE session_receipts RENAME TO session_receipts_v4;
            DROP INDEX session_receipts_expiry;
            CREATE TABLE session_receipts (
                session_id TEXT NOT NULL, operation TEXT NOT NULL, expires INTEGER NOT NULL,
                fingerprint TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('append','resolve','compact')),
                version TEXT, message_id TEXT, deduplicated INTEGER, generation TEXT, created INTEGER,
                summary_text TEXT, summary_message_count INTEGER,
                PRIMARY KEY(session_id,operation),
                CHECK((kind='append' AND version IS NOT NULL AND message_id IS NOT NULL AND deduplicated IN (0,1) AND generation IS NULL AND created IS NULL AND summary_text IS NULL AND summary_message_count IS NULL)
                    OR (kind='resolve' AND version IS NULL AND message_id IS NULL AND deduplicated IS NULL AND generation IS NOT NULL AND created IN (0,1) AND summary_text IS NULL AND summary_message_count IS NULL)
                    OR (kind='compact' AND version IS NULL AND message_id IS NULL AND deduplicated IS NULL AND generation IS NOT NULL AND created IS NULL AND summary_text IS NOT NULL AND summary_message_count>=0)));
            INSERT INTO session_receipts(session_id,operation,expires,fingerprint,kind,version,message_id,deduplicated,generation,created)
                SELECT session_id,operation,expires,fingerprint,kind,version,message_id,deduplicated,generation,created FROM session_receipts_v4;
            DROP TABLE session_receipts_v4;
            CREATE INDEX session_receipts_expiry ON session_receipts(expires);").map_err(sqlite_error)?;
    }
    if version.is_none_or(|version| version < 6) {
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS session_fences (
        session_id TEXT PRIMARY KEY, secret BLOB NOT NULL CHECK(length(secret)=32),
        context_version INTEGER NOT NULL CHECK(context_version>=0));
        INSERT OR IGNORE INTO session_fences SELECT id,randomblob(32),0 FROM sessions;
        CREATE TRIGGER IF NOT EXISTS session_fence_create AFTER INSERT ON sessions BEGIN
            INSERT INTO session_fences VALUES(NEW.id,randomblob(32),0); END;
        CREATE TRIGGER IF NOT EXISTS session_fence_remove AFTER DELETE ON sessions BEGIN
            DELETE FROM session_fences WHERE session_id=OLD.id; END;
        CREATE TRIGGER IF NOT EXISTS session_fence_context AFTER UPDATE ON sessions BEGIN
            SELECT CASE WHEN (SELECT context_version FROM session_fences WHERE session_id=NEW.id)=9223372036854775807
                THEN RAISE(ABORT,'session context version overflow') END;
            UPDATE session_fences SET context_version=context_version+1 WHERE session_id=NEW.id; END;")
        .map_err(sqlite_error)?;
    }
    if version.is_none_or(|version| version < 7) {
        context::migrate(&transaction)?;
    }
    config::migrate(&transaction)?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS session_retention_receipts (
        session_id TEXT NOT NULL, operation TEXT NOT NULL, expires INTEGER NOT NULL,
        fingerprint TEXT NOT NULL, generation TEXT NOT NULL,
        scanned INTEGER NOT NULL CHECK(scanned>=0), deleted INTEGER NOT NULL CHECK(deleted>=0 AND deleted<=scanned),
        dedup_removed INTEGER NOT NULL CHECK(dedup_removed>=0 AND dedup_removed<=deleted),
        next_after INTEGER CHECK(next_after>=0), PRIMARY KEY(session_id,operation));
        CREATE INDEX IF NOT EXISTS session_retention_receipts_expiry ON session_retention_receipts(expires);")
        .map_err(sqlite_error)?;
    transaction
        .execute("UPDATE etas_session_schema SET format=9 WHERE id=1", [])
        .map_err(sqlite_error)?;
    transaction.commit().map_err(sqlite_error)
}

pub(super) fn session_exists(connection: &Connection, session: &str) -> Result<bool, HostError> {
    connection
        .query_row(
            "SELECT 1 FROM sessions WHERE id = ?1",
            params![session],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(sqlite_error)
}
