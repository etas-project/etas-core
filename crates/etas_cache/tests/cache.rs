use etas_cache::{
    ArtifactDependencyGraph, ArtifactFingerprint, ArtifactKey, ArtifactMeta, ArtifactStore,
    CacheError, CachePriority, CachedArtifact, CompressionKind, DiskArtifactBytes,
    DiskArtifactStore, DiskArtifactStoreOptions, DiskArtifactStorePolicy, DiskCacheBudgetPolicy,
    DiskPutStatus, DiskReadOptions, DiskWriteSkipReason, InvalidationSelector, MemoryArtifactStore,
    PayloadCodec, ProjectRevision, TypedArtifactStore,
};
use std::{
    fs,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn meta(dependencies: Vec<ArtifactKey>) -> ArtifactMeta {
    ArtifactMeta::new(
        ProjectRevision(1),
        ArtifactFingerprint::new([7; 32]),
        "test-compiler",
        1,
    )
    .with_dependencies(dependencies)
}

#[test]
fn dependency_graph_invalidates_reverse_dependents_transitively() {
    let source = ArtifactKey::new("frontend", "source", "a.es");
    let parsed = ArtifactKey::new("frontend", "parsed_source", "a.es");
    let hir = ArtifactKey::new("frontend", "hir_module", "A");

    let mut graph = ArtifactDependencyGraph::new();
    graph.add_dependency(parsed.clone(), source.clone());
    graph.add_dependency(hir.clone(), parsed.clone());

    let invalidated = graph.invalidate_from(&[source.clone()]).into_keys();

    assert_eq!(invalidated, vec![source, parsed, hir]);
}

#[test]
fn memory_store_round_trips_typed_artifacts_and_metadata() -> Result<(), CacheError> {
    let key = ArtifactKey::new("frontend", "parsed_source", "a.es");
    let artifact = CachedArtifact {
        key: key.clone(),
        meta: meta(Vec::new()),
        value: "parsed".to_owned(),
    };

    let mut store = MemoryArtifactStore::new();
    store.put(artifact.clone())?;

    assert!(store.contains(&key)?);
    assert_eq!(store.meta(&key)?, Some(artifact.meta.clone()));
    assert_eq!(store.get::<String>(&key)?, Some(artifact));
    assert_eq!(store.get::<u32>(&key)?, None);
    Ok(())
}

#[test]
fn memory_store_invalidates_dependents_without_removing_unrelated_artifacts()
-> Result<(), CacheError> {
    let source = ArtifactKey::new("frontend", "source", "a.es");
    let parsed = ArtifactKey::new("frontend", "parsed_source", "a.es");
    let hir = ArtifactKey::new("frontend", "hir_module", "A");
    let unrelated = ArtifactKey::new("frontend", "parsed_source", "b.es");

    let mut store = MemoryArtifactStore::new();
    store.put(CachedArtifact {
        key: source.clone(),
        meta: meta(Vec::new()),
        value: "source",
    })?;
    store.put(CachedArtifact {
        key: parsed.clone(),
        meta: meta(vec![source.clone()]),
        value: "parsed",
    })?;
    store.put(CachedArtifact {
        key: hir.clone(),
        meta: meta(vec![parsed.clone()]),
        value: "hir",
    })?;
    store.put(CachedArtifact {
        key: unrelated.clone(),
        meta: meta(Vec::new()),
        value: "other",
    })?;

    let report = store.invalidate(InvalidationSelector::Exact(source.clone()))?;

    assert_eq!(report.roots, vec![source.clone()]);
    assert_eq!(report.invalidated, vec![source, parsed, hir]);
    assert!(!store.contains(&ArtifactKey::new("frontend", "source", "a.es"))?);
    assert!(store.contains(&unrelated)?);
    Ok(())
}

#[test]
fn memory_store_can_invalidate_by_namespace() -> Result<(), CacheError> {
    let frontend = ArtifactKey::new("frontend", "parsed_source", "a.es");
    let interpreter = ArtifactKey::new("interpreter", "entry_plan", "main");

    let mut store = MemoryArtifactStore::new();
    store.put(CachedArtifact {
        key: frontend.clone(),
        meta: meta(Vec::new()),
        value: "frontend",
    })?;
    store.put(CachedArtifact {
        key: interpreter.clone(),
        meta: meta(Vec::new()),
        value: "interpreter",
    })?;

    let report = store.invalidate(InvalidationSelector::Namespace(frontend.namespace.clone()))?;

    assert_eq!(report.invalidated, vec![frontend]);
    assert!(store.contains(&interpreter)?);
    Ok(())
}

#[test]
fn disk_store_persists_enveloped_payload_and_dependency_metadata() -> Result<(), CacheError> {
    let root = temp_cache_dir("persist");
    let key = ArtifactKey::new("frontend", "parsed_source", "a.module:part.es");
    let dependency = ArtifactKey::new("frontend", "source", "a.module:part.es");
    let fingerprint = ArtifactFingerprint::new([9; 32]);
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };

    {
        let mut store = DiskArtifactStore::open(&root, options.clone())?;
        let meta = ArtifactMeta::new(ProjectRevision(3), fingerprint, "test-compiler", 1)
            .with_std_version("std-v1")
            .with_options_hash("options-v1")
            .with_dependencies(vec![dependency.clone()]);
        store.put_bytes(DiskArtifactBytes {
            key: key.clone(),
            meta,
            codec: PayloadCodec::Postcard,
            compression: CompressionKind::None,
            payload: b"caller-owned-payload".to_vec(),
        })?;
    }

    let mut reopened = DiskArtifactStore::open(&root, options)?;
    let stored = reopened
        .get_bytes(&DiskReadOptions {
            key: key.clone(),
            fingerprint,
            compiler_version: "test-compiler".to_owned(),
            cache_schema_version: 1,
        })?
        .expect("persisted artifact should be readable");

    assert_eq!(stored.key, key);
    assert_eq!(stored.meta.dependencies, vec![dependency]);
    assert_eq!(stored.meta.std_version.as_deref(), Some("std-v1"));
    assert_eq!(stored.meta.options_hash.as_deref(), Some("options-v1"));
    assert_eq!(stored.codec, PayloadCodec::Postcard);
    assert_eq!(stored.compression, CompressionKind::None);
    assert_eq!(stored.payload, b"caller-owned-payload");
    assert!(stored.meta.payload_hash.is_some());
    assert!(stored.meta.payload_size.is_some());
    Ok(())
}

#[test]
fn disk_store_persists_metadata_only_records_without_payloads() -> Result<(), CacheError> {
    let root = temp_cache_dir("metadata-only");
    let key = ArtifactKey::new("frontend", "source_fingerprint_summary", "project");
    let dependency = ArtifactKey::new("frontend", "source", "src/main.es");
    let fingerprint = ArtifactFingerprint::new([11; 32]);
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let meta = ArtifactMeta::new(ProjectRevision(4), fingerprint, "test-compiler", 1)
        .with_std_version("std-v1")
        .with_options_hash("options-v1")
        .with_dependencies(vec![dependency.clone()]);

    {
        let mut store = DiskArtifactStore::open(&root, options.clone())?;
        let stored = store.put_metadata(key.clone(), meta.clone())?;
        assert_eq!(stored, meta);
        assert!(store.contains(&key)?);
        assert_eq!(final_object_count(&root)?, 0);
    }

    let mut reopened = DiskArtifactStore::open(&root, options)?;
    let stored_meta = reopened
        .meta(&key)?
        .expect("metadata-only artifact should have disk metadata");
    assert_eq!(stored_meta.payload_hash, None);
    assert_eq!(stored_meta.payload_size, None);
    assert_eq!(stored_meta.dependencies, vec![dependency.clone()]);
    assert_eq!(stored_meta.std_version.as_deref(), Some("std-v1"));
    assert_eq!(stored_meta.options_hash.as_deref(), Some("options-v1"));
    assert!(
        reopened
            .get_bytes(&DiskReadOptions {
                key: key.clone(),
                fingerprint,
                compiler_version: "test-compiler".to_owned(),
                cache_schema_version: 1,
            })?
            .is_none(),
        "metadata-only records must not be readable as payload artifacts"
    );
    assert!(
        reopened.contains(&key)?,
        "payload lookup miss must not remove metadata-only records"
    );

    let report = reopened.invalidate(InvalidationSelector::Exact(dependency.clone()))?;
    assert_eq!(report.roots, vec![dependency.clone()]);
    assert_eq!(report.invalidated, vec![dependency, key.clone()]);
    assert!(!reopened.contains(&key)?);
    Ok(())
}

#[test]
fn disk_store_uses_cross_process_visible_metadata_and_payloads() -> Result<(), CacheError> {
    let root = temp_cache_dir("cross-process-visible");
    let key = ArtifactKey::new("frontend", "type_facts", "body-1");
    let fingerprint = ArtifactFingerprint::new([4; 32]);
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };

    let mut writer = DiskArtifactStore::open(&root, options.clone())?;
    writer.put_bytes(DiskArtifactBytes {
        key: key.clone(),
        meta: ArtifactMeta::new(ProjectRevision(1), fingerprint, "test-compiler", 1),
        codec: PayloadCodec::Postcard,
        compression: CompressionKind::None,
        payload: b"writer-payload".to_vec(),
    })?;

    let mut reader = DiskArtifactStore::open(&root, options)?;
    let artifact = reader
        .get_bytes(&DiskReadOptions {
            key,
            fingerprint,
            compiler_version: "test-compiler".to_owned(),
            cache_schema_version: 1,
        })?
        .expect("artifact written by another store should be visible");

    assert_eq!(artifact.payload, b"writer-payload");
    Ok(())
}

#[test]
fn disk_store_concurrent_same_payload_publication_is_idempotent() -> Result<(), CacheError> {
    let root = temp_cache_dir("concurrent-same-payload");
    let key = ArtifactKey::new("frontend", "type_facts", "body-1");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    DiskArtifactStore::open(&root, options.clone())?;
    let (ready_tx, ready_rx) = mpsc::channel();
    let mut handles = Vec::new();
    let mut start_signals = Vec::new();

    for _ in 0..2 {
        let root = root.clone();
        let key = key.clone();
        let options = options.clone();
        let ready_tx = ready_tx.clone();
        let (start_tx, start_rx) = mpsc::channel();
        start_signals.push(start_tx);
        handles.push(std::thread::spawn(move || -> Result<(), CacheError> {
            let mut store = DiskArtifactStore::open(&root, options)?;
            ready_tx
                .send(())
                .expect("main test thread should receive writer readiness");
            start_rx
                .recv()
                .expect("main test thread should release writer start");
            store.put_bytes(DiskArtifactBytes {
                key,
                meta: ArtifactMeta::new(
                    ProjectRevision(1),
                    ArtifactFingerprint::new([5; 32]),
                    "test-compiler",
                    1,
                ),
                codec: PayloadCodec::Postcard,
                compression: CompressionKind::None,
                payload: b"same-payload".to_vec(),
            })?;
            Ok(())
        }));
    }
    drop(ready_tx);
    for _ in 0..2 {
        ready_rx
            .recv()
            .expect("writer should report readiness before publishing");
    }
    for start in start_signals {
        start.send(()).expect("writer should wait for start signal");
    }

    for handle in handles {
        handle.join().expect("writer thread should not panic")?;
    }

    assert_eq!(final_object_count(&root)?, 1);
    assert_eq!(temp_object_count(&root)?, 0);
    Ok(())
}

#[test]
fn disk_store_same_payload_different_envelope_skips_disk_persistence() -> Result<(), CacheError> {
    let root = temp_cache_dir("same-payload-different-envelope");
    let first = ArtifactKey::new("frontend", "type_facts", "body-1");
    let second = ArtifactKey::new("frontend", "type_facts", "body-2");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open(&root, options)?;
    store.put_bytes(DiskArtifactBytes {
        key: first,
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([5; 32]),
            "test-compiler",
            1,
        ),
        codec: PayloadCodec::Postcard,
        compression: CompressionKind::None,
        payload: b"same-payload".to_vec(),
    })?;

    let result = store.put_bytes(DiskArtifactBytes {
        key: second.clone(),
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([6; 32]),
            "test-compiler",
            1,
        ),
        codec: PayloadCodec::Postcard,
        compression: CompressionKind::None,
        payload: b"same-payload".to_vec(),
    });

    assert!(matches!(result, Err(CacheError::Unavailable(_))));
    assert!(store.meta(&second)?.is_none());
    Ok(())
}

#[test]
fn disk_store_write_lock_timeout_is_reported_as_cache_unavailable() -> Result<(), CacheError> {
    let root = temp_cache_dir("write-lock-timeout");
    let key = ArtifactKey::new("frontend", "type_facts", "body-1");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open_with_policy(
        &root,
        options.clone(),
        DiskArtifactStorePolicy {
            busy_timeout: Duration::from_millis(1),
            stale_temp_file_age: Duration::from_secs(6 * 60 * 60),
            ..DiskArtifactStorePolicy::default()
        },
    )?;
    let lock = rusqlite::Connection::open(root.join("v1").join("cache.sqlite"))?;
    lock.execute_batch("BEGIN IMMEDIATE;")?;

    let result = store.put_bytes(DiskArtifactBytes {
        key: key.clone(),
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([6; 32]),
            "test-compiler",
            1,
        ),
        codec: PayloadCodec::Postcard,
        compression: CompressionKind::None,
        payload: b"locked-writer".to_vec(),
    });

    assert!(matches!(result, Err(CacheError::Unavailable(_))));
    lock.execute_batch("ROLLBACK;")?;
    assert!(store.meta(&key)?.is_none());
    Ok(())
}

#[test]
fn disk_store_reports_skipped_write_when_payload_exceeds_policy_max() -> Result<(), CacheError> {
    let root = temp_cache_dir("budget-max-payload");
    let key = ArtifactKey::new("frontend", "type_facts", "body-too-large");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open_with_policy(
        &root,
        options,
        DiskArtifactStorePolicy {
            budget: DiskCacheBudgetPolicy::default().with_max_payload_bytes(4),
            ..DiskArtifactStorePolicy::default()
        },
    )?;

    let report = store.put_bytes_with_report(DiskArtifactBytes {
        key: key.clone(),
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([17; 32]),
            "test-compiler",
            1,
        ),
        codec: PayloadCodec::Postcard,
        compression: CompressionKind::None,
        payload: b"too-large".to_vec(),
    })?;

    assert_eq!(report.key, key);
    assert!(matches!(
        report.status,
        DiskPutStatus::Skipped(DiskWriteSkipReason::PayloadTooLarge {
            max_payload_bytes: 4,
            actual_payload_bytes: 9,
        })
    ));
    assert!(store.meta(&report.key)?.is_none());
    assert_eq!(final_object_count(&root)?, 0);
    Ok(())
}

#[test]
fn disk_store_telemetry_records_artifact_kind_events() -> Result<(), CacheError> {
    let root = temp_cache_dir("telemetry");
    let first = ArtifactKey::new("frontend", "type_facts", "first");
    let second = ArtifactKey::new("frontend", "type_facts", "second");
    let third = ArtifactKey::new("frontend", "type_facts", "third");
    let too_large = ArtifactKey::new("frontend", "type_facts", "too-large");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open_with_policy(
        &root,
        options,
        DiskArtifactStorePolicy {
            budget: DiskCacheBudgetPolicy::default()
                .with_max_project_bytes(8)
                .with_max_payload_bytes(32),
            ..DiskArtifactStorePolicy::default()
        },
    )?;

    store.record_compute_time(&first, Duration::from_millis(7));
    assert!(
        store
            .get_bytes(&DiskReadOptions {
                key: first.clone(),
                fingerprint: ArtifactFingerprint::new([30; 32]),
                compiler_version: "test-compiler".to_owned(),
                cache_schema_version: 1,
            })?
            .is_none()
    );
    put_disk_payload(&mut store, first.clone(), Vec::new(), [30; 32], b"1111")?;
    store
        .get_bytes(&DiskReadOptions {
            key: first.clone(),
            fingerprint: ArtifactFingerprint::new([30; 32]),
            compiler_version: "test-compiler".to_owned(),
            cache_schema_version: 1,
        })?
        .expect("stored payload should be readable");
    let report = store.put_bytes_with_report(DiskArtifactBytes {
        key: too_large,
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([31; 32]),
            "test-compiler",
            1,
        ),
        codec: PayloadCodec::Postcard,
        compression: CompressionKind::None,
        payload: vec![0; 64],
    })?;
    assert!(matches!(
        report.status,
        DiskPutStatus::Skipped(DiskWriteSkipReason::PayloadTooLarge { .. })
    ));
    put_disk_payload(&mut store, second, Vec::new(), [32; 32], b"2222")?;
    put_disk_payload(&mut store, third, Vec::new(), [33; 32], b"3333")?;

    let telemetry = store.telemetry();
    let entry = telemetry
        .artifact_kind(&first)
        .expect("frontend type_facts telemetry should exist");
    assert_eq!(entry.compute_count, 1);
    assert_eq!(entry.compute_time, Duration::from_millis(7));
    assert_eq!(entry.hit_count, 1);
    assert_eq!(entry.miss_count, 1);
    assert_eq!(entry.skipped_write_count, 1);
    assert_eq!(entry.eviction_count, 1);
    assert_eq!(entry.compressed_bytes, 12);
    Ok(())
}

#[test]
fn disk_store_project_budget_evicts_lower_priority_artifacts() -> Result<(), CacheError> {
    let root = temp_cache_dir("budget-project-priority");
    let low = ArtifactKey::new("frontend", "low", "a");
    let high = ArtifactKey::new("frontend", "high", "b");
    let normal = ArtifactKey::new("frontend", "normal", "c");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open_with_policy(
        &root,
        options,
        DiskArtifactStorePolicy {
            budget: DiskCacheBudgetPolicy::default()
                .with_max_project_bytes(12)
                .with_kind_priority("frontend", "low", CachePriority::Low)
                .with_kind_priority("frontend", "high", CachePriority::High),
            ..DiskArtifactStorePolicy::default()
        },
    )?;

    put_disk_payload(&mut store, low.clone(), Vec::new(), [18; 32], b"111111")?;
    put_disk_payload(&mut store, high.clone(), Vec::new(), [19; 32], b"222222")?;
    put_disk_payload(&mut store, normal.clone(), Vec::new(), [20; 32], b"333333")?;

    assert!(store.meta(&low)?.is_none());
    assert!(store.meta(&high)?.is_some());
    assert!(store.meta(&normal)?.is_some());
    assert_eq!(final_object_count(&root)?, 2);
    Ok(())
}

#[test]
fn disk_store_project_budget_evicts_least_recently_used_with_same_priority()
-> Result<(), CacheError> {
    let root = temp_cache_dir("budget-project-last-used");
    let first = ArtifactKey::new("frontend", "type_facts", "first");
    let second = ArtifactKey::new("frontend", "type_facts", "second");
    let third = ArtifactKey::new("frontend", "type_facts", "third");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open_with_policy(
        &root,
        options,
        DiskArtifactStorePolicy {
            budget: DiskCacheBudgetPolicy::default().with_max_project_bytes(12),
            ..DiskArtifactStorePolicy::default()
        },
    )?;

    put_disk_payload(&mut store, first.clone(), Vec::new(), [21; 32], b"111111")?;
    put_disk_payload(&mut store, second.clone(), Vec::new(), [22; 32], b"222222")?;
    set_last_used_at(&root, &first, 100)?;
    set_last_used_at(&root, &second, 1)?;
    put_disk_payload(&mut store, third.clone(), Vec::new(), [23; 32], b"333333")?;

    assert!(store.meta(&first)?.is_some());
    assert!(store.meta(&second)?.is_none());
    assert!(store.meta(&third)?.is_some());
    assert_eq!(final_object_count(&root)?, 2);
    Ok(())
}

#[test]
fn disk_store_namespace_budget_evicts_only_that_namespace() -> Result<(), CacheError> {
    let root = temp_cache_dir("budget-namespace");
    let frontend_old = ArtifactKey::new("frontend", "type_facts", "old");
    let frontend_new = ArtifactKey::new("frontend", "type_facts", "new");
    let interpreter = ArtifactKey::new("interpreter", "entry_plan", "main");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open_with_policy(
        &root,
        options,
        DiskArtifactStorePolicy {
            budget: DiskCacheBudgetPolicy::default().with_namespace_budget("frontend", 6),
            ..DiskArtifactStorePolicy::default()
        },
    )?;

    put_disk_payload(
        &mut store,
        interpreter.clone(),
        Vec::new(),
        [24; 32],
        b"outside",
    )?;
    put_disk_payload(
        &mut store,
        frontend_old.clone(),
        Vec::new(),
        [25; 32],
        b"inside",
    )?;
    set_last_used_at(&root, &frontend_old, 1)?;
    put_disk_payload(
        &mut store,
        frontend_new.clone(),
        Vec::new(),
        [26; 32],
        b"newest",
    )?;

    assert!(store.meta(&interpreter)?.is_some());
    assert!(store.meta(&frontend_old)?.is_none());
    assert!(store.meta(&frontend_new)?.is_some());
    Ok(())
}

#[test]
fn disk_store_gc_keeps_active_temp_objects() -> Result<(), CacheError> {
    let root = temp_cache_dir("gc-active-temp");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let store = DiskArtifactStore::open(&root, options)?;
    let temp_dir = store.root().join("objects").join("aa");
    fs::create_dir_all(&temp_dir)?;
    let temp = temp_dir.join(".active.tmp");
    fs::write(&temp, b"in-progress")?;

    let removed = store.gc_unreachable_objects()?;

    assert!(removed.is_empty());
    assert!(temp.exists());
    Ok(())
}

#[test]
fn disk_store_returns_dependency_metadata_in_artifact_key_order() -> Result<(), CacheError> {
    let root = temp_cache_dir("dependency-order");
    let key = ArtifactKey::new("frontend", "checked_project", "project");
    let short_encoded_kind = ArtifactKey::new("frontend", "type_facts", "project");
    let long_encoded_kind = ArtifactKey::new("frontend", "artifact_manifest", "project");
    let mut dependencies = vec![short_encoded_kind.clone(), long_encoded_kind.clone()];
    dependencies.sort();
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };

    {
        let mut store = DiskArtifactStore::open(&root, options.clone())?;
        store.put_bytes(DiskArtifactBytes {
            key: key.clone(),
            meta: ArtifactMeta::new(
                ProjectRevision(3),
                ArtifactFingerprint::new([9; 32]),
                "test-compiler",
                1,
            )
            .with_dependencies(vec![short_encoded_kind, long_encoded_kind]),
            codec: PayloadCodec::Postcard,
            compression: CompressionKind::None,
            payload: b"manifest".to_vec(),
        })?;
    }

    let reopened = DiskArtifactStore::open(&root, options)?;
    assert_eq!(
        reopened.meta(&key)?.expect("artifact meta").dependencies,
        dependencies
    );
    Ok(())
}

#[test]
fn disk_store_rejects_fingerprint_mismatch_instead_of_cache_miss() -> Result<(), CacheError> {
    let root = temp_cache_dir("fingerprint-mismatch");
    let key = ArtifactKey::new("frontend", "type_facts", "body-1");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open(&root, options)?;
    store.put_bytes(DiskArtifactBytes {
        key: key.clone(),
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([1; 32]),
            "test-compiler",
            1,
        ),
        codec: PayloadCodec::Bincode2,
        compression: CompressionKind::Zstd,
        payload: b"facts".to_vec(),
    })?;

    let result = store.get_bytes(&DiskReadOptions {
        key,
        fingerprint: ArtifactFingerprint::new([2; 32]),
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    });

    assert!(matches!(result, Err(CacheError::Integrity(_))));
    Ok(())
}

#[test]
fn disk_store_invalidates_reverse_dependencies_and_gc_removes_unreachable_objects()
-> Result<(), CacheError> {
    let root = temp_cache_dir("invalidate");
    let source = ArtifactKey::new("frontend", "source", "a.es");
    let parsed = ArtifactKey::new("frontend", "parsed_source", "a.es");
    let hir = ArtifactKey::new("frontend", "hir_module", "A");
    let options = DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    };
    let mut store = DiskArtifactStore::open(&root, options)?;
    put_disk_payload(&mut store, source.clone(), Vec::new(), [1; 32], b"source")?;
    put_disk_payload(
        &mut store,
        parsed.clone(),
        vec![source.clone()],
        [2; 32],
        b"parsed",
    )?;
    put_disk_payload(
        &mut store,
        hir.clone(),
        vec![parsed.clone()],
        [3; 32],
        b"hir",
    )?;

    let report = store.invalidate(InvalidationSelector::Exact(source.clone()))?;

    assert_eq!(
        report.invalidated,
        vec![source.clone(), parsed.clone(), hir.clone()]
    );
    assert!(!store.contains(&source)?);
    assert!(!store.contains(&parsed)?);
    assert!(!store.contains(&hir)?);
    let removed = store.gc_unreachable_objects()?;
    assert_eq!(removed.len(), 3);
    Ok(())
}

fn put_disk_payload(
    store: &mut DiskArtifactStore,
    key: ArtifactKey,
    dependencies: Vec<ArtifactKey>,
    fingerprint: [u8; 32],
    payload: &[u8],
) -> Result<(), CacheError> {
    store.put_bytes(DiskArtifactBytes {
        key,
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new(fingerprint),
            "test-compiler",
            1,
        )
        .with_dependencies(dependencies),
        codec: PayloadCodec::Postcard,
        compression: CompressionKind::None,
        payload: payload.to_vec(),
    })?;
    Ok(())
}

fn temp_cache_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "etas-cache-test-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp cache dir");
    path
}

fn set_last_used_at(
    root: &std::path::Path,
    key: &ArtifactKey,
    last_used_at: u64,
) -> Result<(), CacheError> {
    let connection = rusqlite::Connection::open(root.join("v1").join("cache.sqlite"))?;
    connection.execute(
        "
        UPDATE artifacts
        SET last_used_at = ?1
        WHERE namespace = ?2 AND kind = ?3 AND unit = ?4
        ",
        rusqlite::params![
            last_used_at,
            key.namespace.as_str(),
            key.kind.as_str(),
            key.unit.as_str()
        ],
    )?;
    Ok(())
}

fn final_object_count(root: &std::path::Path) -> Result<usize, CacheError> {
    object_file_count(root, |name| name.ends_with(".bin"))
}

fn temp_object_count(root: &std::path::Path) -> Result<usize, CacheError> {
    object_file_count(root, |name| name.starts_with('.') && name.ends_with(".tmp"))
}

fn object_file_count(
    root: &std::path::Path,
    matches_name: impl Fn(&str) -> bool,
) -> Result<usize, CacheError> {
    let objects = root.join("v1").join("objects");
    if !objects.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for shard in fs::read_dir(objects)? {
        let shard = shard?;
        if !shard.file_type()?.is_dir() {
            continue;
        }
        for object in fs::read_dir(shard.path())? {
            let object = object?;
            if object.file_type()?.is_file()
                && object.file_name().to_str().is_some_and(&matches_name)
            {
                count += 1;
            }
        }
    }
    Ok(count)
}
