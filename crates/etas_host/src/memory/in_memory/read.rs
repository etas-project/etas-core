use super::*;
use crate::memory::{
    cursor::CursorFence,
    query::*,
    selection::{TopK, entry_parts_size, exhausted, page_limit},
};
use crate::{ExecutionBudget, MemoryCursor, MemoryEntry, MemoryQuery};
impl InMemoryMemoryClient {
    fn select(
        &self,
        store: &StoreRef,
        cursor: Option<MemoryCursor>,
        limit: Option<u32>,
        predicate: Option<HostValue>,
        embedding: Option<Vec<f32>>,
        budget: &ExecutionBudget,
    ) -> Result<MemoryResult, HostError> {
        let limit = page_limit(limit, &self.limits)?;
        let stores = self.stores.read().map_err(lock_error)?;
        let Some(state) = stores.get(&store_key(store)) else {
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
        let scope = scope_identity("volatile", &store.region.stable_id, &store.path);
        let fence = CursorFence {
            scope: &scope,
            generation: state.generation.as_str(),
            revision: state.revision,
        };
        let after = fence.resume(cursor.as_ref(), &self.limits)?;
        let start = after.map_or(std::ops::Bound::Unbounded, std::ops::Bound::Excluded);
        let mut entries = Vec::new();
        let mut top = TopK::new(limit);
        let mut bytes = 0usize;
        let mut more = false;
        let mut last = None;
        for (scanned, (key, entry)) in state
            .entries
            .range((start, std::ops::Bound::Unbounded))
            .enumerate()
        {
            budget.check_time()?;
            if scanned == self.limits.max_scan_rows {
                return Err(exhausted());
            }
            if !memory_query_matches(&entry.key, &entry.value, predicate.as_ref()) {
                continue;
            }
            let candidate_size =
                entry_parts_size(&entry.key, &entry.value, &entry.version, &self.limits)?;
            if let Some(embedding) = &embedding {
                if let Some(score) = extract_embedding(&entry.value)
                    .and_then(|values| cosine_similarity(embedding, values))
                {
                    top.push(
                        score,
                        key,
                        candidate_size,
                        || MemoryEntry {
                            key: entry.key.clone(),
                            value: entry.value.clone(),
                            version: entry.version.clone(),
                        },
                        &self.limits,
                    )?;
                }
            } else {
                let next = bytes.checked_add(candidate_size).ok_or_else(exhausted)?;
                if entries.len() == limit || next > self.limits.max_result_bytes {
                    if entries.is_empty() {
                        return Err(exhausted());
                    }
                    more = true;
                    break;
                }
                bytes = next;
                last = Some(key.as_str());
                entries.push(MemoryEntry {
                    key: entry.key.clone(),
                    value: entry.value.clone(),
                    version: entry.version.clone(),
                });
            }
        }
        let cursor = if more {
            Some(fence.after(last.ok_or_else(exhausted)?, &self.limits)?)
        } else {
            None
        };
        if embedding.is_some() {
            entries = top.finish();
        }
        Ok(MemoryResult::Entries { entries, cursor })
    }
    pub(super) fn scan(
        &self,
        store: &StoreRef,
        cursor: Option<MemoryCursor>,
        limit: Option<u32>,
        budget: &ExecutionBudget,
    ) -> Result<MemoryResult, HostError> {
        self.select(store, cursor, limit, None, None, budget)
    }
    pub(super) fn query(
        &self,
        store: &StoreRef,
        query: MemoryQuery,
        limit: Option<u32>,
        budget: &ExecutionBudget,
    ) -> Result<MemoryResult, HostError> {
        if !query.order_by.is_empty() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "ordered memory query is unsupported",
            ));
        }
        self.select(store, None, limit, query.predicate, None, budget)
    }
    pub(super) fn vector_search(
        &self,
        store: &StoreRef,
        embedding: Vec<f32>,
        limit: u32,
        filter: Option<HostValue>,
        budget: &ExecutionBudget,
    ) -> Result<MemoryResult, HostError> {
        validate_query_embedding(&embedding)?;
        self.select(store, None, Some(limit), filter, Some(embedding), budget)
    }
}
