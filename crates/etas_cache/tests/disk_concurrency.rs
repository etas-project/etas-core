use etas_cache::{
    ArtifactFingerprint, ArtifactKey, ArtifactMeta, ArtifactStore, CacheError, CompressionKind,
    ContentHash, DiskArtifactBytes, DiskArtifactStore, DiskArtifactStoreOptions,
    DiskArtifactStorePolicy, DiskReadOptions, PayloadCodec, ProjectRevision,
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const CHILD_ROOT_ENV: &str = "ETAS_CACHE_TEST_ROOT";
const CHILD_UNIT_ENV: &str = "ETAS_CACHE_TEST_UNIT";
const CHILD_FINGERPRINT_ENV: &str = "ETAS_CACHE_TEST_FINGERPRINT_BYTE";
const CHILD_PAYLOAD_ENV: &str = "ETAS_CACHE_TEST_PAYLOAD";
const CHILD_DEPENDENCY_ENV: &str = "ETAS_CACHE_TEST_DEPENDENCY_UNIT";

#[test]
#[ignore = "subprocess helper invoked by disk concurrency tests"]
fn subprocess_writer_helper() -> Result<(), CacheError> {
    let root =
        std::env::var(CHILD_ROOT_ENV).expect("subprocess writer root path should be provided");
    let unit = std::env::var(CHILD_UNIT_ENV).unwrap_or_else(|_| "body-subprocess".to_owned());
    let fingerprint = std::env::var(CHILD_FINGERPRINT_ENV)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(8);
    let payload =
        std::env::var(CHILD_PAYLOAD_ENV).unwrap_or_else(|_| "subprocess-payload".to_owned());
    let dependencies = std::env::var(CHILD_DEPENDENCY_ENV)
        .ok()
        .map(|unit| vec![ArtifactKey::new("frontend", "source", unit)])
        .unwrap_or_default();

    let mut store = DiskArtifactStore::open(PathBuf::from(root), store_options())?;
    put_payload(
        &mut store,
        ArtifactKey::new("frontend", "type_facts", unit),
        dependencies,
        fingerprint,
        payload.as_bytes(),
    )?;
    Ok(())
}

#[test]
fn disk_store_reuses_payload_written_by_subprocess() -> Result<(), CacheError> {
    let root = temp_cache_dir("subprocess-writer");
    let mut child = spawn_writer(&root, "body-subprocess", 8, "from-subprocess", None)?;
    wait_for_success(&mut child, "subprocess writer")?;

    let mut reader = DiskArtifactStore::open(&root, store_options())?;
    let stored = reader
        .get_bytes(&DiskReadOptions {
            key: ArtifactKey::new("frontend", "type_facts", "body-subprocess"),
            fingerprint: ArtifactFingerprint::new([8; 32]),
            compiler_version: "test-compiler".to_owned(),
            cache_schema_version: 1,
        })?
        .expect("subprocess-written artifact should be visible");

    assert_eq!(stored.payload, b"from-subprocess");
    Ok(())
}

#[test]
fn disk_store_concurrent_subprocess_writers_keep_dependency_rows_consistent()
-> Result<(), CacheError> {
    let root = temp_cache_dir("subprocess-writer-contention");
    DiskArtifactStore::open(&root, store_options())?;
    let mut first = spawn_writer(&root, "body-a", 11, "payload-a", Some("source-a"))?;
    let mut second = spawn_writer(&root, "body-b", 12, "payload-b", Some("source-b"))?;

    wait_for_success(&mut first, "first subprocess writer")?;
    wait_for_success(&mut second, "second subprocess writer")?;

    let reader = DiskArtifactStore::open(&root, store_options())?;
    assert_eq!(
        reader
            .meta(&ArtifactKey::new("frontend", "type_facts", "body-a"))?
            .expect("body-a metadata")
            .dependencies,
        vec![ArtifactKey::new("frontend", "source", "source-a")]
    );
    assert_eq!(
        reader
            .meta(&ArtifactKey::new("frontend", "type_facts", "body-b"))?
            .expect("body-b metadata")
            .dependencies,
        vec![ArtifactKey::new("frontend", "source", "source-b")]
    );
    assert_eq!(final_object_count(&root)?, 2);
    assert_eq!(temp_object_count(&root)?, 0);
    Ok(())
}

#[test]
fn disk_store_multiple_simultaneous_readers_share_committed_artifact() -> Result<(), CacheError> {
    let root = temp_cache_dir("simultaneous-readers");
    let key = ArtifactKey::new("frontend", "type_facts", "body-shared-reader");
    {
        let mut writer = DiskArtifactStore::open(&root, store_options())?;
        put_payload(&mut writer, key.clone(), Vec::new(), 16, b"shared-reader")?;
    }

    let (ready_tx, ready_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    let mut starts = Vec::new();
    for _ in 0..4 {
        let root = root.clone();
        let key = key.clone();
        let ready_tx = ready_tx.clone();
        let result_tx = result_tx.clone();
        let (start_tx, start_rx) = mpsc::channel();
        starts.push(start_tx);
        thread::spawn(move || {
            let result = (|| -> Result<(), CacheError> {
                let mut reader = DiskArtifactStore::open(&root, store_options())?;
                ready_tx
                    .send(())
                    .expect("main test thread should receive reader readiness");
                start_rx
                    .recv()
                    .expect("main test thread should release reader start");
                for _ in 0..20 {
                    let stored = reader
                        .get_bytes(&DiskReadOptions {
                            key: key.clone(),
                            fingerprint: ArtifactFingerprint::new([16; 32]),
                            compiler_version: "test-compiler".to_owned(),
                            cache_schema_version: 1,
                        })?
                        .expect("committed artifact should remain readable");
                    assert_eq!(stored.payload, b"shared-reader");
                }
                Ok(())
            })();
            result_tx
                .send(result)
                .expect("main test thread should receive reader result");
        });
    }
    drop(ready_tx);
    drop(result_tx);

    for _ in 0..4 {
        ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| CacheError::Unavailable("reader readiness timed out".to_owned()))?;
    }
    for start in starts {
        start.send(()).expect("reader should wait for start signal");
    }
    for _ in 0..4 {
        result_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| CacheError::Unavailable("reader result timed out".to_owned()))??;
    }
    Ok(())
}

#[test]
fn disk_store_invalidation_during_concurrent_reads_is_consistent() -> Result<(), CacheError> {
    let root = temp_cache_dir("invalidate-during-read");
    let source = ArtifactKey::new("frontend", "source", "source-a");
    let parsed = ArtifactKey::new("frontend", "parsed_source", "source-a");
    {
        let mut writer = DiskArtifactStore::open(&root, store_options())?;
        put_payload(&mut writer, source.clone(), Vec::new(), 17, b"source")?;
        put_payload(
            &mut writer,
            parsed.clone(),
            vec![source.clone()],
            18,
            b"parsed",
        )?;
    }

    let (ready_tx, ready_rx) = mpsc::channel();
    let (start_tx, start_rx) = mpsc::channel();
    let (result_tx, result_rx) = mpsc::channel();
    {
        let root = root.clone();
        let parsed = parsed.clone();
        thread::spawn(move || {
            let result = (|| -> Result<(), CacheError> {
                let mut reader = DiskArtifactStore::open(&root, store_options())?;
                ready_tx
                    .send(())
                    .expect("main test thread should receive reader readiness");
                start_rx
                    .recv()
                    .expect("main test thread should release reader start");
                for _ in 0..100 {
                    let stored = reader.get_bytes(&DiskReadOptions {
                        key: parsed.clone(),
                        fingerprint: ArtifactFingerprint::new([18; 32]),
                        compiler_version: "test-compiler".to_owned(),
                        cache_schema_version: 1,
                    })?;
                    if let Some(stored) = stored {
                        assert_eq!(stored.payload, b"parsed");
                    }
                    thread::sleep(Duration::from_millis(1));
                }
                Ok(())
            })();
            result_tx
                .send(result)
                .expect("main test thread should receive reader result");
        });
    }

    ready_rx
        .recv_timeout(Duration::from_secs(10))
        .map_err(|_| CacheError::Unavailable("reader readiness timed out".to_owned()))?;
    start_tx
        .send(())
        .expect("reader should wait for start signal");
    let mut invalidator = DiskArtifactStore::open(&root, store_options())?;
    let report = invalidator.invalidate(etas_cache::InvalidationSelector::Exact(source.clone()))?;

    result_rx
        .recv_timeout(Duration::from_secs(10))
        .map_err(|_| CacheError::Unavailable("reader result timed out".to_owned()))??;
    assert_eq!(report.invalidated, vec![source.clone(), parsed.clone()]);
    assert!(!invalidator.contains(&source)?);
    assert!(!invalidator.contains(&parsed)?);
    Ok(())
}

#[test]
fn disk_store_gc_keeps_reachable_objects_while_another_store_is_open() -> Result<(), CacheError> {
    let root = temp_cache_dir("gc-open-store");
    let key = ArtifactKey::new("frontend", "type_facts", "body-open-reader");
    let mut writer = DiskArtifactStore::open(&root, store_options())?;
    put_payload(&mut writer, key.clone(), Vec::new(), 19, b"reachable")?;
    let mut reader = DiskArtifactStore::open(&root, store_options())?;

    let removed = writer.gc_unreachable_objects()?;

    assert!(removed.is_empty());
    let stored = reader
        .get_bytes(&DiskReadOptions {
            key,
            fingerprint: ArtifactFingerprint::new([19; 32]),
            compiler_version: "test-compiler".to_owned(),
            cache_schema_version: 1,
        })?
        .expect("reachable committed object should remain readable");
    assert_eq!(stored.payload, b"reachable");
    Ok(())
}

#[test]
fn disk_store_get_ignores_touch_lock_contention() -> Result<(), CacheError> {
    let root = temp_cache_dir("touch-lock-contention");
    let key = ArtifactKey::new("frontend", "type_facts", "body-touch-lock");
    let fingerprint = ArtifactFingerprint::new([13; 32]);
    {
        let mut writer = DiskArtifactStore::open(&root, store_options())?;
        put_payload(&mut writer, key.clone(), Vec::new(), 13, b"touch-lock")?;
    }

    let reader_policy = DiskArtifactStorePolicy {
        busy_timeout: Duration::from_millis(1),
        stale_temp_file_age: Duration::from_secs(6 * 60 * 60),
        ..DiskArtifactStorePolicy::default()
    };
    let mut reader = DiskArtifactStore::open_with_policy(&root, store_options(), reader_policy)?;
    let lock = rusqlite::Connection::open(root.join("v1").join("cache.sqlite"))?;
    lock.busy_timeout(Duration::from_millis(1))?;
    lock.execute_batch("BEGIN IMMEDIATE;")?;

    assert!(reader.contains(&key)?);
    assert!(reader.meta(&key)?.is_some());
    let stored = reader
        .get_bytes(&DiskReadOptions {
            key: key.clone(),
            fingerprint,
            compiler_version: "test-compiler".to_owned(),
            cache_schema_version: 1,
        })?
        .expect("payload read should not fail when only touch metadata is locked");
    assert_eq!(stored.payload, b"touch-lock");

    lock.execute_batch("ROLLBACK;")?;
    Ok(())
}

#[test]
fn disk_store_missing_blob_is_cache_miss_and_removes_metadata() -> Result<(), CacheError> {
    let root = temp_cache_dir("missing-blob");
    let key = ArtifactKey::new("frontend", "type_facts", "body-missing");
    let mut store = DiskArtifactStore::open(&root, store_options())?;
    put_payload(&mut store, key.clone(), Vec::new(), 14, b"missing-blob")?;
    let hash = store
        .meta(&key)?
        .expect("metadata before removing object")
        .payload_hash
        .expect("payload hash");
    fs::remove_file(object_path(&root, &hash))?;

    assert!(
        store
            .get_bytes(&DiskReadOptions {
                key: key.clone(),
                fingerprint: ArtifactFingerprint::new([14; 32]),
                compiler_version: "test-compiler".to_owned(),
                cache_schema_version: 1,
            })?
            .is_none()
    );
    assert!(store.meta(&key)?.is_none());
    Ok(())
}

#[test]
fn disk_store_corrupt_blob_is_cache_miss_and_removes_metadata() -> Result<(), CacheError> {
    let root = temp_cache_dir("corrupt-blob");
    let key = ArtifactKey::new("frontend", "type_facts", "body-corrupt");
    let mut store = DiskArtifactStore::open(&root, store_options())?;
    put_payload(&mut store, key.clone(), Vec::new(), 15, b"corrupt-blob")?;
    let hash = store
        .meta(&key)?
        .expect("metadata before corrupting object")
        .payload_hash
        .expect("payload hash");
    fs::write(object_path(&root, &hash), b"not a cache envelope")?;

    assert!(
        store
            .get_bytes(&DiskReadOptions {
                key: key.clone(),
                fingerprint: ArtifactFingerprint::new([15; 32]),
                compiler_version: "test-compiler".to_owned(),
                cache_schema_version: 1,
            })?
            .is_none()
    );
    assert!(store.meta(&key)?.is_none());
    Ok(())
}

#[test]
fn disk_store_gc_removes_stale_temp_objects_only_after_threshold() -> Result<(), CacheError> {
    let root = temp_cache_dir("gc-stale-temp");
    let store = DiskArtifactStore::open_with_policy(
        &root,
        store_options(),
        DiskArtifactStorePolicy {
            busy_timeout: Duration::from_millis(750),
            stale_temp_file_age: Duration::ZERO,
            ..DiskArtifactStorePolicy::default()
        },
    )?;
    let temp_dir = store.root().join("objects").join("aa");
    fs::create_dir_all(&temp_dir)?;
    let stale = temp_dir.join(".stale.tmp");
    fs::write(&stale, b"abandoned temp object")?;

    let removed = store.gc_unreachable_objects()?;

    assert_eq!(removed, vec![stale.clone()]);
    assert!(!stale.exists());
    Ok(())
}

#[test]
fn disk_store_gc_removes_crash_orphan_final_objects() -> Result<(), CacheError> {
    let root = temp_cache_dir("gc-orphan-final-object");
    let store = DiskArtifactStore::open(&root, store_options())?;
    let orphan_dir = store.root().join("objects").join("aa");
    fs::create_dir_all(&orphan_dir)?;
    let orphan =
        orphan_dir.join("aa00000000000000000000000000000000000000000000000000000000000000.bin");
    fs::write(&orphan, b"orphan object")?;

    let removed = store.gc_unreachable_objects()?;

    assert_eq!(removed, vec![orphan.clone()]);
    assert!(!orphan.exists());
    Ok(())
}

#[test]
fn disk_store_reuses_existing_final_object_without_rewriting_it() -> Result<(), CacheError> {
    let root = temp_cache_dir("reuse-final-object");
    let key = ArtifactKey::new("frontend", "type_facts", "body-final-object");
    let mut store = DiskArtifactStore::open(&root, store_options())?;
    put_payload(&mut store, key.clone(), Vec::new(), 20, b"immutable-final")?;
    let hash = store
        .meta(&key)?
        .expect("metadata after first write")
        .payload_hash
        .expect("payload hash");
    let path = object_path(&root, &hash);
    let original = fs::read(&path)?;
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions)?;

    put_payload(&mut store, key, Vec::new(), 20, b"immutable-final")?;

    assert_eq!(fs::read(&path)?, original);
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_readonly(false);
    fs::set_permissions(&path, permissions)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn disk_store_gc_does_not_fail_when_orphan_object_cannot_be_removed() -> Result<(), CacheError> {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_cache_dir("gc-remove-failure");
    let store = DiskArtifactStore::open(&root, store_options())?;
    let orphan_dir = store.root().join("objects").join("aa");
    fs::create_dir_all(&orphan_dir)?;
    let orphan =
        orphan_dir.join("aa11111111111111111111111111111111111111111111111111111111111111.bin");
    fs::write(&orphan, b"orphan object")?;
    let original_permissions = fs::metadata(&orphan_dir)?.permissions();
    fs::set_permissions(&orphan_dir, fs::Permissions::from_mode(0o555))?;

    let removed = store.gc_unreachable_objects()?;

    assert!(removed.is_empty());
    assert!(orphan.exists());
    fs::set_permissions(&orphan_dir, original_permissions)?;
    fs::remove_file(orphan)?;
    Ok(())
}

fn put_payload(
    store: &mut DiskArtifactStore,
    key: ArtifactKey,
    dependencies: Vec<ArtifactKey>,
    fingerprint_byte: u8,
    payload: &[u8],
) -> Result<(), CacheError> {
    store.put_bytes(DiskArtifactBytes {
        key,
        meta: ArtifactMeta::new(
            ProjectRevision(1),
            ArtifactFingerprint::new([fingerprint_byte; 32]),
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

fn spawn_writer(
    root: &Path,
    unit: &str,
    fingerprint_byte: u8,
    payload: &str,
    dependency_unit: Option<&str>,
) -> Result<Child, CacheError> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .arg("--exact")
        .arg("subprocess_writer_helper")
        .arg("--ignored")
        .arg("--test-threads=1")
        .env(CHILD_ROOT_ENV, root)
        .env(CHILD_UNIT_ENV, unit)
        .env(CHILD_FINGERPRINT_ENV, fingerprint_byte.to_string())
        .env(CHILD_PAYLOAD_ENV, payload)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dependency_unit) = dependency_unit {
        command.env(CHILD_DEPENDENCY_ENV, dependency_unit);
    }
    Ok(command.spawn()?)
}

fn wait_for_success(child: &mut Child, label: &str) -> Result<(), CacheError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            return Err(CacheError::Integrity(format!(
                "{label} exited unsuccessfully: {status}"
            )));
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(CacheError::Unavailable(format!("{label} timed out")));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn store_options() -> DiskArtifactStoreOptions {
    DiskArtifactStoreOptions {
        compiler_version: "test-compiler".to_owned(),
        cache_schema_version: 1,
    }
}

fn temp_cache_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "etas-cache-concurrency-test-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp cache dir");
    path
}

fn object_path(root: &Path, payload_hash: &ContentHash) -> PathBuf {
    let hash = payload_hash.to_string();
    root.join("v1")
        .join("objects")
        .join(&hash[..2])
        .join(format!("{hash}.bin"))
}

fn final_object_count(root: &Path) -> Result<usize, CacheError> {
    object_file_count(root, |name| name.ends_with(".bin"))
}

fn temp_object_count(root: &Path) -> Result<usize, CacheError> {
    object_file_count(root, |name| name.starts_with('.') && name.ends_with(".tmp"))
}

fn object_file_count(
    root: &Path,
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
