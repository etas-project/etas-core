use super::*;
use crate::{ArtifactFingerprint, ProjectRevision};
use std::sync::Arc;

fn artifact<T>(kind: &str, unit: &str, value: T) -> CachedArtifact<T> {
    CachedArtifact {
        key: ArtifactKey::new("test", kind, unit),
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([1; 32]),
            "test",
            1,
        ),
        value,
    }
}

#[test]
fn shared_payload_is_counted_once_but_metadata_and_footprints_are_not_free() {
    let mut store = MemoryArtifactStore::new();
    let value = Arc::new(vec![0_u8; 100_000]);
    store.put(artifact("first", "1", value.clone())).unwrap();
    let one = store.memory_usage();
    store.put(artifact("second", "2", value.clone())).unwrap();
    let two = store.memory_usage();
    assert_eq!(
        two.payload_estimated_bytes - one.payload_estimated_bytes,
        size_of::<Arc<Vec<u8>>>() as u64
    );
    assert!(two.bookkeeping_estimated_bytes > one.bookkeeping_estimated_bytes);
    assert!(
        two.by_kind
            .iter()
            .all(|kind| kind.referenced_payload_estimated_bytes > 100_000)
    );
    assert!(two.estimated_retained_bytes < 150_000);
    store
        .remove(&ArtifactKey::new("test", "first", "1"))
        .unwrap();
    assert_eq!(
        store.memory_usage().payload_estimated_bytes,
        one.payload_estimated_bytes
    );
    assert_eq!(Arc::strong_count(&value), 2);
    store
        .invalidate(InvalidationSelector::Namespace(crate::CacheNamespace::new(
            "test",
        )))
        .unwrap();
    assert_eq!(store.memory_usage().estimated_retained_bytes, 0);
    assert_eq!(Arc::strong_count(&value), 1);
}

#[test]
fn empty_reserved_payload_cannot_bypass_budget() {
    let mut store = MemoryArtifactStore::with_byte_budget(4096);
    let mut large = artifact("large", "1", Vec::<u8>::with_capacity(100_000));
    large.meta.payload_size = Some(0);
    assert_eq!(
        store.put(large).unwrap(),
        crate::ArtifactPutStatus::SkippedMemoryBudget
    );
    let usage = store.memory_usage();
    assert_eq!(usage.entries, 0);
    assert_eq!(usage.estimated_retained_bytes, 0);
    assert_eq!(usage.rejected_insertions, 1);
}

#[test]
fn an_oversized_admission_does_not_flush_smaller_valid_entries() {
    let mut store = MemoryArtifactStore::with_byte_budget(8192);
    let small = artifact("small", "1", vec![0_u8; 100]);
    let key = small.key.clone();
    store.put(small).unwrap();
    store
        .put(artifact("large", "2", vec![0_u8; 100_000]))
        .unwrap();
    assert!(store.contains(&key).unwrap());
    assert_eq!(store.memory_usage().entries, 1);
    assert_eq!(store.memory_usage().rejected_insertions, 1);
}

#[test]
fn reads_refresh_eviction_order_without_a_growing_access_log() {
    let mut store = MemoryArtifactStore::new();
    for unit in ["1", "2", "3"] {
        store
            .put(artifact("value", unit, vec![0_u8; 20_000]))
            .unwrap();
    }
    let first = ArtifactKey::new("test", "value", "1");
    store.get::<Vec<u8>>(&first).unwrap().unwrap();
    store.set_byte_budget(50_000).unwrap();
    assert!(store.contains(&first).unwrap());
    assert!(
        !store
            .contains(&ArtifactKey::new("test", "value", "2"))
            .unwrap()
    );
    assert!(
        store
            .contains(&ArtifactKey::new("test", "value", "3"))
            .unwrap()
    );
}

#[test]
fn least_recent_entry_and_its_dependents_are_evicted_together() {
    let mut store = MemoryArtifactStore::new();
    let first = artifact("value", "1", vec![1_u8; 20_000]);
    let root = first.key.clone();
    store.put(first).unwrap();
    let mut dependent = artifact("value", "2", vec![2_u8; 20_000]);
    dependent.meta.dependencies.push(root.clone());
    let dependent_key = dependent.key.clone();
    store.put(dependent).unwrap();
    let third = artifact("value", "3", vec![3_u8; 20_000]);
    let third_key = third.key.clone();
    store.put(third).unwrap();
    store.set_byte_budget(50_000).unwrap();
    assert!(!store.contains(&root).unwrap());
    assert!(!store.contains(&dependent_key).unwrap());
    assert!(store.contains(&third_key).unwrap());
    assert!(store.memory_usage().estimated_retained_bytes <= 50_000);
}

#[test]
fn zero_budget_retains_no_entries_and_read_touches_are_bounded() {
    let mut store = MemoryArtifactStore::new();
    let value = artifact("value", "1", vec![0_u8; 100]);
    let key = value.key.clone();
    store.put(value).unwrap();
    let before = store.memory_usage();
    for _ in 0..1000 {
        assert!(store.get::<Vec<u8>>(&key).unwrap().is_some());
    }
    assert_eq!(store.memory_usage(), before);
    store.set_byte_budget(0).unwrap();
    store.put(artifact("value", "2", vec![0_u8; 100])).unwrap();
    assert_eq!(store.memory_usage().entries, 0);
    assert_eq!(store.memory_usage().estimated_retained_bytes, 0);
}

#[test]
fn dependencies_on_an_uncached_root_remain_invalidation_roots() {
    let mut store = MemoryArtifactStore::new();
    let root = ArtifactKey::new("source", "input", "missing");
    let mut value = artifact("value", "1", 42_u32);
    value.meta.dependencies.push(root.clone());
    let key = value.key.clone();
    store.put(value).unwrap();
    store.invalidate(InvalidationSelector::Exact(root)).unwrap();
    assert!(!store.contains(&key).unwrap());
}
