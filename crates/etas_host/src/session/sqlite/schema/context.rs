use super::*;

pub(super) fn migrate(transaction: &rusqlite::Transaction<'_>) -> Result<(), HostError> {
    transaction.execute_batch("ALTER TABLE session_receipts RENAME TO session_receipts_v6;
        DROP INDEX session_receipts_expiry;
        CREATE TABLE session_receipts (
            session_id TEXT NOT NULL, operation TEXT NOT NULL, expires INTEGER NOT NULL,
            fingerprint TEXT NOT NULL, kind TEXT NOT NULL CHECK(kind IN ('append','resolve','compact','context')),
            version TEXT, message_id TEXT, deduplicated INTEGER, generation TEXT, created INTEGER,
            summary_text TEXT, summary_message_count INTEGER, context_status TEXT, context_version INTEGER,
            PRIMARY KEY(session_id,operation),
            CHECK((kind='append' AND version IS NOT NULL AND message_id IS NOT NULL AND deduplicated IN (0,1) AND generation IS NULL AND created IS NULL AND summary_text IS NULL AND summary_message_count IS NULL AND context_status IS NULL AND context_version IS NULL)
                OR (kind='resolve' AND version IS NULL AND message_id IS NULL AND deduplicated IS NULL AND generation IS NOT NULL AND created IN (0,1) AND summary_text IS NULL AND summary_message_count IS NULL AND context_status IS NULL AND context_version IS NULL)
                OR (kind='compact' AND version IS NULL AND message_id IS NULL AND deduplicated IS NULL AND generation IS NOT NULL AND created IS NULL AND summary_text IS NOT NULL AND summary_message_count>=0 AND context_status IS NULL AND context_version IS NULL)
                OR (kind='context' AND version IS NULL AND message_id IS NULL AND deduplicated IS NULL AND created IS NULL AND summary_text IS NULL AND summary_message_count IS NULL AND context_status IS NOT NULL AND
                    ((context_status='committed' AND generation IS NOT NULL AND typeof(context_version)='integer' AND context_version>0) OR (context_status='stale' AND generation IS NULL AND context_version IS NULL)))));
        INSERT INTO session_receipts(session_id,operation,expires,fingerprint,kind,version,message_id,deduplicated,generation,created,summary_text,summary_message_count)
            SELECT session_id,operation,expires,fingerprint,kind,version,message_id,deduplicated,generation,created,summary_text,summary_message_count FROM session_receipts_v6;
        DROP TABLE session_receipts_v6;
        CREATE INDEX session_receipts_expiry ON session_receipts(expires);
        CREATE TABLE IF NOT EXISTS session_contexts (
            session_id TEXT PRIMARY KEY, content TEXT NOT NULL, fence TEXT NOT NULL,
            version INTEGER NOT NULL CHECK(typeof(version)='integer' AND version>0));
        CREATE TRIGGER IF NOT EXISTS session_context_insert AFTER INSERT ON session_contexts BEGIN
            SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM session_fences WHERE session_id=NEW.session_id AND context_version=NEW.version-1)
                THEN RAISE(ABORT,'invalid session context version') END;
            UPDATE session_fences SET context_version=NEW.version WHERE session_id=NEW.session_id;
            UPDATE session_history_generations SET generation=lower(hex(randomblob(16))) WHERE session_id=NEW.session_id;
        END;
        CREATE TRIGGER IF NOT EXISTS session_context_update AFTER UPDATE ON session_contexts BEGIN
            SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM session_fences WHERE session_id=NEW.session_id AND context_version=NEW.version-1)
                THEN RAISE(ABORT,'invalid session context version') END;
            UPDATE session_fences SET context_version=NEW.version WHERE session_id=NEW.session_id;
            UPDATE session_history_generations SET generation=lower(hex(randomblob(16))) WHERE session_id=NEW.session_id;
        END;
        CREATE TRIGGER IF NOT EXISTS session_context_delete AFTER DELETE ON session_contexts BEGIN
            SELECT CASE WHEN (SELECT context_version FROM session_fences WHERE session_id=OLD.session_id)=9223372036854775807
                THEN RAISE(ABORT,'session context version overflow') END;
            UPDATE session_fences SET context_version=context_version+1 WHERE session_id=OLD.session_id;
            UPDATE session_history_generations SET generation=lower(hex(randomblob(16))) WHERE session_id=OLD.session_id;
        END;
        DROP TRIGGER IF EXISTS session_fence_context;

        CREATE TRIGGER IF NOT EXISTS session_context_remove AFTER DELETE ON sessions BEGIN
            DELETE FROM session_contexts WHERE session_id=OLD.id; END;")
        .map_err(sqlite_error)?;
    install_legacy_trigger(transaction)
}

pub(super) fn install_legacy_trigger(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<(), HostError> {
    transaction.execute_batch("CREATE TRIGGER IF NOT EXISTS session_context_legacy_replacement AFTER UPDATE ON sessions BEGIN
            SELECT CASE WHEN NOT EXISTS(SELECT 1 FROM session_contexts WHERE session_id=NEW.id)
                AND (SELECT context_version FROM session_fences WHERE session_id=NEW.id)=9223372036854775807
                THEN RAISE(ABORT,'session context version overflow') END;
            UPDATE session_fences SET context_version=context_version+1 WHERE session_id=NEW.id
                AND NOT EXISTS(SELECT 1 FROM session_contexts WHERE session_id=NEW.id);
            DELETE FROM session_contexts WHERE session_id=NEW.id; END;").map_err(sqlite_error)
}
