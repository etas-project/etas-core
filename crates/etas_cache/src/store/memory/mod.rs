mod accounting;
#[cfg(test)]
mod tests;

use std::{any::Any, cell::Cell, collections::HashMap};

use etas_core::memory::{MemoryFootprint, RetainedSize, hash_table_bytes};

use crate::{
    ArtifactDependencyGraph, ArtifactKey, ArtifactMeta, CacheResult, CachedArtifact,
    InvalidationReport, InvalidationSelector,
    store::{ArtifactStore, TypedArtifactStore},
};
use accounting::{Accounting, KindUsage, record_kind};
pub use accounting::{ArtifactKindMemoryUsage, DEFAULT_MEMORY_CACHE_BYTES, MemoryCacheUsage};

pub struct MemoryArtifactStore {
    artifacts: HashMap<ArtifactKey, MemoryArtifact>,
    accounting: Accounting,
    budget_bytes: u64,
    clock: Cell<u64>,
    evicted_artifacts: u64,
    rejected_insertions: u64,
}

struct MemoryArtifact {
    meta: ArtifactMeta,
    value: Box<dyn Any>,
    footprint: MemoryFootprint,
    bookkeeping: u64,
    accessed: Cell<u64>,
}

impl Default for MemoryArtifactStore {
    fn default() -> Self {
        Self::with_byte_budget(DEFAULT_MEMORY_CACHE_BYTES)
    }
}

impl MemoryArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_byte_budget(budget_bytes: u64) -> Self {
        Self {
            artifacts: HashMap::new(),
            accounting: Accounting::default(),
            budget_bytes,
            clock: Cell::new(0),
            evicted_artifacts: 0,
            rejected_insertions: 0,
        }
    }

    pub fn set_byte_budget(&mut self, bytes: u64) -> CacheResult<()> {
        self.budget_bytes = bytes;
        self.enforce_budget()
    }

    pub fn memory_usage(&self) -> MemoryCacheUsage {
        let mut kinds = KindUsage::new();
        for (key, entry) in &self.artifacts {
            record_kind(&mut kinds, key, &entry.footprint, entry.bookkeeping);
        }
        MemoryCacheUsage {
            budget_bytes: self.budget_bytes,
            estimated_retained_bytes: self.estimated_bytes(),
            payload_estimated_bytes: self.accounting.payload_bytes,
            bookkeeping_estimated_bytes: self.bookkeeping_bytes(),
            entries: self.artifacts.len(),
            evicted_artifacts: self.evicted_artifacts,
            rejected_insertions: self.rejected_insertions,
            by_kind: kinds.into_values().map(|(usage, _)| usage).collect(),
        }
    }

    // ArtifactMeta is the single dependency source. Do not retain a duplicate
    // graph, or forget edges when a dependency payload is not retained.
    pub fn dependency_graph(&self) -> ArtifactDependencyGraph {
        let mut graph = ArtifactDependencyGraph::new();
        for (key, entry) in &self.artifacts {
            graph.set_dependencies(key.clone(), entry.meta.dependencies.clone());
        }
        graph
    }

    fn touch(&self, entry: &MemoryArtifact) {
        self.clock.set(self.clock.get().saturating_add(1));
        entry.accessed.set(self.clock.get());
    }

    fn bookkeeping_bytes(&self) -> u64 {
        self.accounting
            .bookkeeping_bytes()
            .saturating_add(hash_table_bytes::<(ArtifactKey, MemoryArtifact)>(
                self.artifacts.capacity(),
            ) as u64)
    }

    fn estimated_bytes(&self) -> u64 {
        self.accounting
            .payload_bytes
            .saturating_add(self.bookkeeping_bytes())
    }

    fn remove_entry(&mut self, key: &ArtifactKey) -> CacheResult<()> {
        if let Some(entry) = self.artifacts.remove(key) {
            self.accounting
                .remove(&entry.footprint, entry.bookkeeping)?;
        }
        if self.artifacts.capacity() > 4 * self.artifacts.len() {
            self.artifacts.shrink_to_fit();
        }
        Ok(())
    }

    fn enforce_budget(&mut self) -> CacheResult<()> {
        while self.estimated_bytes() > self.budget_bytes {
            let Some(victim) = self
                .artifacts
                .iter()
                .min_by_key(|(key, entry)| (entry.accessed.get(), *key))
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            let invalidated = self
                .dependency_graph()
                .invalidate_from(&[victim])
                .into_keys();
            for key in invalidated {
                if self.artifacts.contains_key(&key) {
                    self.evicted_artifacts += 1;
                }
                self.remove_entry(&key)?;
            }
        }
        Ok(())
    }

    fn roots_for(&self, selector: InvalidationSelector) -> Vec<ArtifactKey> {
        match selector {
            InvalidationSelector::Exact(key) => vec![key],
            InvalidationSelector::Roots(keys) => keys,
            InvalidationSelector::Namespace(namespace) => self
                .artifacts
                .keys()
                .filter(|key| key.namespace == namespace)
                .cloned()
                .collect(),
            InvalidationSelector::Kind { namespace, kind } => self
                .artifacts
                .keys()
                .filter(|key| key.namespace == namespace && key.kind == kind)
                .cloned()
                .collect(),
        }
    }
}

impl ArtifactStore for MemoryArtifactStore {
    fn contains(&self, key: &ArtifactKey) -> CacheResult<bool> {
        Ok(self.artifacts.contains_key(key))
    }

    fn meta(&self, key: &ArtifactKey) -> CacheResult<Option<ArtifactMeta>> {
        Ok(self.artifacts.get(key).map(|entry| {
            self.touch(entry);
            entry.meta.clone()
        }))
    }

    fn remove(&mut self, key: &ArtifactKey) -> CacheResult<()> {
        self.invalidate(InvalidationSelector::Exact(key.clone()))?;
        Ok(())
    }

    fn invalidate(&mut self, selector: InvalidationSelector) -> CacheResult<InvalidationReport> {
        let roots = self.roots_for(selector);
        let invalidated = self.dependency_graph().invalidate_from(&roots).into_keys();
        for key in &invalidated {
            self.remove_entry(key)?;
        }
        Ok(InvalidationReport { roots, invalidated })
    }
}

impl TypedArtifactStore for MemoryArtifactStore {
    fn get<T: Clone + 'static>(&self, key: &ArtifactKey) -> CacheResult<Option<CachedArtifact<T>>> {
        let Some(entry) = self.artifacts.get(key) else {
            return Ok(None);
        };
        let Some(value) = entry.value.downcast_ref::<T>() else {
            return Ok(None);
        };
        self.touch(entry);
        Ok(Some(CachedArtifact {
            key: key.clone(),
            meta: entry.meta.clone(),
            value: value.clone(),
        }))
    }

    fn put<T: Clone + RetainedSize + 'static>(
        &mut self,
        artifact: CachedArtifact<T>,
    ) -> CacheResult<crate::ArtifactPutStatus> {
        self.remove_entry(&artifact.key)?;
        if self.budget_bytes == 0 {
            self.rejected_insertions += 1;
            return Ok(crate::ArtifactPutStatus::SkippedMemoryBudget);
        }
        let value = Box::new(artifact.value);
        let footprint = MemoryFootprint::for_boxed(&*value);
        let bookkeeping = MemoryFootprint::of(&artifact.key)
            .estimated_bytes()
            .saturating_add(MemoryFootprint::of(&artifact.meta).estimated_bytes())
            .saturating_add(footprint.accounting_bytes());
        // A payload that cannot fit on its own must not flush useful entries.
        if footprint.estimated_bytes().saturating_add(bookkeeping) > self.budget_bytes {
            self.rejected_insertions = self.rejected_insertions.saturating_add(1);
            return Ok(crate::ArtifactPutStatus::SkippedMemoryBudget);
        }
        self.accounting.add(&footprint, bookkeeping);
        self.clock.set(self.clock.get().saturating_add(1));
        let key = artifact.key.clone();
        self.artifacts.insert(
            artifact.key,
            MemoryArtifact {
                meta: artifact.meta,
                value,
                footprint,
                bookkeeping,
                accessed: Cell::new(self.clock.get()),
            },
        );
        self.enforce_budget()?;
        if !self.artifacts.contains_key(&key) {
            self.rejected_insertions += 1;
            return Ok(crate::ArtifactPutStatus::SkippedMemoryBudget);
        }
        Ok(crate::ArtifactPutStatus::Stored)
    }
}
