use super::*;
use crate::session::context::fence::{HistoryKey, HistoryState};

pub(super) fn state(connection: &Connection, session: &str) -> Result<HistoryState, HostError> {
    connection
        .query_row(
            "SELECT s.generation,h.generation,q.last_ordinal,f.context_version,f.secret
         FROM session_storage_generations s
         JOIN session_history_generations h ON h.session_id=s.session_id
         JOIN session_sequences q ON q.session_id=s.session_id
         JOIN session_fences f ON f.session_id=s.session_id WHERE s.session_id=?1",
            [session],
            |row| {
                let incarnation = bounded_text(row, 0, 32)?;
                let revision = bounded_text(row, 1, 32)?;
                let upper: i64 = row.get(2)?;
                let version: i64 = row.get(3)?;
                let key = row.get_ref(4)?.as_blob().map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Blob,
                        Box::new(error),
                    )
                })?;
                let invalid = || {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Blob,
                        Box::new(HostError::new(
                            HostErrorCode::SchemaMismatch,
                            "invalid session fence state",
                        )),
                    )
                };
                if upper < -1 || upper == i64::MAX || version < 0 {
                    return Err(invalid());
                }
                Ok(HistoryState {
                    incarnation,
                    revision,
                    upper,
                    context_version: version as u64,
                    key: HistoryKey::from_bytes(key).map_err(|_| invalid())?,
                })
            },
        )
        .optional()
        .map_err(sqlite_error)?
        .ok_or_else(|| HostError::new(HostErrorCode::SchemaMismatch, "missing session fence state"))
}
