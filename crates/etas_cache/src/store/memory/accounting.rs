use std::collections::{BTreeMap, HashMap, HashSet};

use etas_core::memory::{AllocationId, MemoryFootprint, hash_table_bytes};

use crate::{ArtifactKindKey, CacheNamespace};

pub const DEFAULT_MEMORY_CACHE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArtifactKindMemoryUsage {
    pub namespace: String,
    pub kind: String,
    pub entries: usize,
    /// De-duplicated within this kind; may overlap other kinds or live revisions.
    pub referenced_payload_estimated_bytes: u64,
    pub entry_bookkeeping_estimated_bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemoryCacheUsage {
    pub budget_bytes: u64,
    /// Owned storage/capacity estimates, not allocator samples or RSS.
    pub estimated_retained_bytes: u64,
    pub payload_estimated_bytes: u64,
    pub bookkeeping_estimated_bytes: u64,
    pub entries: usize,
    pub evicted_artifacts: u64,
    pub rejected_insertions: u64,
    pub by_kind: Vec<ArtifactKindMemoryUsage>,
}

pub(super) struct AllocationCharge {
    bytes: u64,
    references: usize,
}

#[derive(Default)]
pub(super) struct Accounting {
    allocations: HashMap<AllocationId, AllocationCharge>,
    pub payload_bytes: u64,
    pub entry_bytes: u64,
}

impl Accounting {
    pub fn add(&mut self, footprint: &MemoryFootprint, bookkeeping: u64) {
        self.entry_bytes = self.entry_bytes.saturating_add(bookkeeping);
        for allocation in &footprint.allocations {
            let charge = self
                .allocations
                .entry(allocation.id)
                .or_insert(AllocationCharge {
                    bytes: 0,
                    references: 0,
                });
            let extra = allocation.estimated_bytes.saturating_sub(charge.bytes);
            charge.bytes = charge.bytes.max(allocation.estimated_bytes);
            charge.references += 1;
            self.payload_bytes = self.payload_bytes.saturating_add(extra);
        }
    }

    pub fn remove(
        &mut self,
        footprint: &MemoryFootprint,
        bookkeeping: u64,
    ) -> crate::CacheResult<()> {
        if bookkeeping > self.entry_bytes
            || footprint.allocations.iter().any(|allocation| {
                self.allocations
                    .get(&allocation.id)
                    .is_none_or(|charge| charge.references == 0)
            })
        {
            return Err(crate::CacheError::Integrity(
                "memory cache allocation ledger is inconsistent".to_owned(),
            ));
        }
        self.entry_bytes -= bookkeeping;
        for allocation in &footprint.allocations {
            let charge = self.allocations.get_mut(&allocation.id).ok_or_else(|| {
                crate::CacheError::Integrity("missing cache allocation charge".to_owned())
            })?;
            charge.references -= 1;
            if charge.references == 0 {
                self.payload_bytes -= charge.bytes;
                self.allocations.remove(&allocation.id);
            }
        }
        if self.allocations.capacity() > 4 * self.allocations.len() {
            self.allocations.shrink_to_fit();
        }
        Ok(())
    }

    pub fn bookkeeping_bytes(&self) -> u64 {
        self.entry_bytes
            .saturating_add(hash_table_bytes::<(AllocationId, AllocationCharge)>(
                self.allocations.capacity(),
            ) as u64)
    }
}

pub(super) type KindUsage =
    BTreeMap<(CacheNamespace, ArtifactKindKey), (ArtifactKindMemoryUsage, HashSet<AllocationId>)>;

pub(super) fn record_kind(
    kinds: &mut KindUsage,
    key: &crate::ArtifactKey,
    footprint: &MemoryFootprint,
    bookkeeping: u64,
) {
    let (usage, seen) = kinds
        .entry((key.namespace.clone(), key.kind.clone()))
        .or_insert_with(|| {
            (
                ArtifactKindMemoryUsage {
                    namespace: key.namespace.as_str().to_owned(),
                    kind: key.kind.as_str().to_owned(),
                    ..Default::default()
                },
                HashSet::new(),
            )
        });
    usage.entries += 1;
    usage.entry_bookkeeping_estimated_bytes += bookkeeping;
    for allocation in &footprint.allocations {
        if seen.insert(allocation.id) {
            usage.referenced_payload_estimated_bytes += allocation.estimated_bytes;
        }
    }
}
