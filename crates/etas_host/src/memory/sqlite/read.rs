use super::*;
use rusqlite::params;
impl MemoryDatabase<'_> {
    pub(super) fn get(
        &mut self,
        store: &StoreRef,
        key: HostValue,
    ) -> Result<MemoryResult, HostError> {
        let key = encode_host_value(&key, self.limits)?;
        let path = store_path_key(store)?;
        let tx = self.connection.transaction().map_err(sqlite_error)?;
        let state = schema::state(&tx, store, &path)?;
        let mut statement=tx.prepare("SELECT value_json,version FROM memory_entries WHERE region=?1 AND path=?2 AND key_json=?3").map_err(sqlite_error)?;
        let mut rows = statement
            .query(params![store.region.stable_id, path, key])
            .map_err(sqlite_error)?;
        let result = if let Some(row) = rows.next().map_err(sqlite_error)? {
            let raw = row
                .get_ref(0)
                .map_err(sqlite_error)?
                .as_str()
                .map_err(|_| {
                    HostError::new(HostErrorCode::SchemaMismatch, "stored value is not text")
                })?;
            let value = decode_host_value(raw, self.limits)?;
            let version = state
                .ok_or_else(|| {
                    HostError::new(HostErrorCode::SchemaMismatch, "entry has no Store metadata")
                })?
                .version(row.get(1).map_err(sqlite_error)?)?;
            crate::memory::selection::validate_value_result(&value, &version, self.limits)?;
            MemoryResult::Value { value, version }
        } else {
            MemoryResult::None
        };
        drop(rows);
        drop(statement);
        tx.commit().map_err(sqlite_error)?;
        Ok(result)
    }
}
