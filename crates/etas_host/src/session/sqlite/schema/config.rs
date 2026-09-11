use crate::{HostError, HostErrorCode};

pub(super) fn migrate(transaction: &rusqlite::Transaction<'_>) -> Result<(), HostError> {
    let incompatible: bool = transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sessions WHERE CASE
            WHEN NOT json_valid(config_json) THEN 1
            WHEN json_type(config_json,'$.compaction') IS NULL THEN 0
            WHEN json_type(config_json,'$.compaction') = 'object' THEN CASE
                WHEN json(json_extract(config_json,'$.compaction')) = '{\"kind\":\"None\"}' THEN 0
                ELSE 1 END
            ELSE 1 END)",
            [],
            |row| row.get(0),
        )
        .map_err(super::super::sqlite_error)?;
    if incompatible {
        return Err(HostError::new(
            HostErrorCode::SchemaMismatch,
            "stored SessionConfig.compaction is obsolete; migrate automatic summarization to application history_page/prepare_context/publish_context before reopening",
        ));
    }
    // Only the disabled legacy option is equivalent to the new contract.
    // Keep messages, published context and receipt identities unchanged.
    // This format-only rewrite is not a replacement of published content.
    // Keep the legacy-summary replacement hook out of it, in this same transaction.
    transaction
        .execute_batch("DROP TRIGGER IF EXISTS session_context_legacy_replacement")
        .map_err(super::super::sqlite_error)?;
    transaction
        .execute(
            "UPDATE sessions SET config_json=json_remove(config_json,'$.compaction')
         WHERE json_type(config_json,'$.compaction') IS NOT NULL",
            [],
        )
        .map_err(super::super::sqlite_error)?;
    super::context::install_legacy_trigger(transaction)?;
    Ok(())
}
