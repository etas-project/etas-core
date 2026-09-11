use crate::{HostError, HostErrorCode, MemoryEntry, StorageLimits};
use std::{cmp::Ordering, collections::BinaryHeap};

#[cfg(test)]
mod tests;

pub(super) fn validate_value_result(
    value: &crate::HostValue,
    version: &crate::MemoryVersion,
    limits: &StorageLimits,
) -> Result<(), HostError> {
    if limits
        .value_size(value)?
        .checked_add(version.as_token().len())
        .is_none_or(|bytes| bytes > limits.max_result_bytes)
    {
        return Err(exhausted());
    }
    Ok(())
}
pub(super) fn page_limit(limit: Option<u32>, limits: &StorageLimits) -> Result<usize, HostError> {
    let limit = limit.map_or(limits.max_page_entries.min(100), |limit| limit as usize);
    if limit == 0 || limit > limits.max_page_entries {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "memory page limit is outside configured bounds",
        ));
    }
    Ok(limit)
}
pub(super) fn entry_size(entry: &MemoryEntry, limits: &StorageLimits) -> Result<usize, HostError> {
    entry_parts_size(&entry.key, &entry.value, &entry.version, limits)
}
pub(super) fn entry_parts_size(
    key: &crate::HostValue,
    value: &crate::HostValue,
    version: &crate::MemoryVersion,
    limits: &StorageLimits,
) -> Result<usize, HostError> {
    limits
        .value_size(key)?
        .checked_add(limits.value_size(value)?)
        .and_then(|size| size.checked_add(version.as_token().len()))
        .ok_or_else(exhausted)
}
pub(super) fn exhausted() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "memory selection exceeds storage limits",
    )
}
struct Candidate {
    score: f64,
    key: String,
    entry: MemoryEntry,
    bytes: usize,
}
impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // The heap root is the worst retained result.
        other
            .score
            .total_cmp(&self.score)
            .then_with(|| self.key.cmp(&other.key))
    }
}
pub(super) struct TopK {
    heap: BinaryHeap<Candidate>,
    limit: usize,
    bytes: usize,
}
impl TopK {
    pub fn new(limit: usize) -> Self {
        Self {
            heap: BinaryHeap::new(),
            limit,
            bytes: 0,
        }
    }
    pub fn push(
        &mut self,
        score: f64,
        key: &str,
        entry_bytes: usize,
        entry: impl FnOnce() -> MemoryEntry,
        limits: &StorageLimits,
    ) -> Result<(), HostError> {
        let bytes = entry_bytes.checked_add(key.len()).ok_or_else(exhausted)?;
        let removed_bytes = if self.heap.len() == self.limit {
            let worst = self.heap.peek().ok_or_else(exhausted)?;
            if worst
                .score
                .total_cmp(&score)
                .then_with(|| key.cmp(&worst.key))
                != Ordering::Less
            {
                return Ok(());
            }
            worst.bytes
        } else {
            0
        };
        let retained = self
            .bytes
            .checked_sub(removed_bytes)
            .and_then(|used| used.checked_add(bytes))
            .ok_or_else(exhausted)?;
        if retained > limits.max_result_bytes {
            return Err(exhausted());
        }
        if self.heap.len() == self.limit {
            self.heap.pop().ok_or_else(exhausted)?;
        }
        self.bytes = retained;
        self.heap.push(Candidate {
            score,
            key: key.to_owned(),
            entry: entry(),
            bytes,
        });
        Ok(())
    }
    pub fn finish(self) -> Vec<MemoryEntry> {
        self.heap
            .into_sorted_vec()
            .into_iter()
            .map(|entry| entry.entry)
            .collect()
    }
}
