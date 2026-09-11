use super::*;
use crate::memory::{
    cursor::CursorFence,
    query::*,
    selection::{TopK, entry_size, exhausted, page_limit},
};
use rusqlite::params;
impl MemoryDatabase<'_> {
    pub(super) fn select(
        &mut self,
        store: &StoreRef,
        cursor: Option<MemoryCursor>,
        limit: Option<u32>,
        predicate: Option<HostValue>,
        embedding: Option<Vec<f32>>,
    ) -> Result<MemoryResult, HostError> {
        let limit = page_limit(limit, self.limits)?;
        let store_path = store_path_key(store)?;
        let transaction = self.connection.transaction().map_err(sqlite_error)?;
        let state = schema::state(&transaction, store, &store_path)?;
        let Some(state) = state else {
            if cursor.is_some() {
                return Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "memory cursor Store no longer exists",
                ));
            }
            return Ok(MemoryResult::Entries {
                entries: Vec::new(),
                cursor: None,
            });
        };
        let fence = CursorFence {
            scope: &state.scope,
            generation: state.generation.as_str(),
            revision: state.revision,
        };
        let after = fence.resume(cursor.as_ref(), self.limits)?;
        let mut statement = transaction
            .prepare(
                "SELECT key_json,value_json,version FROM memory_entries
             WHERE region=?1 AND path=?2 AND (?3 IS NULL OR key_json>?3)
             ORDER BY key_json LIMIT ?4",
            )
            .map_err(sqlite_error)?;
        let mut rows = statement
            .query(params![
                store.region.stable_id,
                store_path,
                after,
                i64::try_from(self.limits.max_scan_rows)
                    .map_err(|_| exhausted())?
                    .checked_add(1)
                    .ok_or_else(exhausted)?
            ])
            .map_err(sqlite_error)?;
        let mut entries = Vec::new();
        let mut top = TopK::new(limit);
        let mut result_bytes = 0usize;
        let mut scanned = 0usize;
        let mut last = None::<String>;
        let mut more = false;
        while let Some(row) = rows.next().map_err(sqlite_error)? {
            if let Some(operation) = self.operation {
                operation.check()?;
            }
            scanned += 1;
            if scanned > self.limits.max_scan_rows {
                return Err(exhausted());
            }
            let key = row
                .get_ref(0)
                .map_err(sqlite_error)?
                .as_str()
                .map_err(|_| {
                    HostError::new(HostErrorCode::SchemaMismatch, "stored key is not text")
                })?;
            let value = row
                .get_ref(1)
                .map_err(sqlite_error)?
                .as_str()
                .map_err(|_| {
                    HostError::new(HostErrorCode::SchemaMismatch, "stored value is not text")
                })?;
            if key.len() > self.limits.max_value_bytes || value.len() > self.limits.max_value_bytes
            {
                return Err(exhausted());
            }
            let entry = MemoryEntry {
                key: decode_host_value(key, self.limits)?,
                value: decode_host_value(value, self.limits)?,
                version: state.version(row.get(2).map_err(sqlite_error)?)?,
            };
            let bytes = entry_size(&entry, self.limits)?;
            if !memory_query_matches(&entry.key, &entry.value, predicate.as_ref()) {
                continue;
            }
            if let Some(embedding) = &embedding {
                if let Some(score) = extract_embedding(&entry.value)
                    .and_then(|candidate| cosine_similarity(embedding, candidate))
                {
                    top.push(score, key, bytes, || entry, self.limits)?;
                }
            } else {
                let next_bytes = result_bytes.checked_add(bytes).ok_or_else(exhausted)?;
                if entries.len() == limit || next_bytes > self.limits.max_result_bytes {
                    if entries.is_empty() {
                        return Err(exhausted());
                    }
                    more = true;
                    break;
                }
                result_bytes = next_bytes;
                last = Some(key.to_owned());
                entries.push(entry);
            }
        }
        let cursor = if more {
            Some(fence.after(last.as_deref().ok_or_else(exhausted)?, self.limits)?)
        } else {
            None
        };
        if embedding.is_some() {
            entries = top.finish();
        }
        drop(rows);
        drop(statement);
        transaction.commit().map_err(sqlite_error)?;
        Ok(MemoryResult::Entries { entries, cursor })
    }
    pub(super) fn scan(
        &mut self,
        store: &StoreRef,
        cursor: Option<MemoryCursor>,
        limit: Option<u32>,
    ) -> Result<MemoryResult, HostError> {
        self.select(store, cursor, limit, None, None)
    }
}
