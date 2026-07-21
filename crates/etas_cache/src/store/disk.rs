use std::{
    cell::RefCell,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, ErrorCode, OptionalExtension, Transaction, params};

use crate::{
    ArtifactEnvelopeHeader, ArtifactFingerprint, ArtifactKey, ArtifactMeta, CacheError,
    CacheResult, CacheTelemetry, CompressionKind, ContentHash, InvalidationReport,
    InvalidationSelector, PayloadCodec, ProjectRevision,
    policy::{CachePriority, DiskCacheBudgetPolicy},
    serialize::{compress_payload, decompress_payload},
    store::ArtifactStore,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskArtifactStoreOptions {
    pub compiler_version: String,
    pub cache_schema_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskArtifactStorePolicy {
    pub busy_timeout: Duration,
    pub stale_temp_file_age: Duration,
    pub budget: DiskCacheBudgetPolicy,
}

impl Default for DiskArtifactStorePolicy {
    fn default() -> Self {
        Self {
            busy_timeout: Duration::from_millis(750),
            stale_temp_file_age: Duration::from_secs(6 * 60 * 60),
            budget: DiskCacheBudgetPolicy::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskReadOptions {
    pub key: ArtifactKey,
    pub fingerprint: ArtifactFingerprint,
    pub compiler_version: String,
    pub cache_schema_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskArtifactBytes {
    pub key: ArtifactKey,
    pub meta: ArtifactMeta,
    pub codec: PayloadCodec,
    pub compression: CompressionKind,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskPutReport {
    pub key: ArtifactKey,
    pub status: DiskPutStatus,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiskPutStatus {
    Stored(ArtifactMeta),
    Skipped(DiskWriteSkipReason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiskWriteSkipReason {
    PayloadTooLarge {
        max_payload_bytes: u64,
        actual_payload_bytes: u64,
    },
    ProjectBudgetTooSmall {
        max_project_bytes: u64,
        actual_payload_bytes: u64,
    },
    NamespaceBudgetTooSmall {
        namespace: String,
        max_namespace_bytes: u64,
        actual_payload_bytes: u64,
    },
}

impl DiskWriteSkipReason {
    fn message(&self) -> &'static str {
        match self {
            Self::PayloadTooLarge { .. } => {
                "cache write skipped because payload exceeds policy max_payload_bytes"
            }
            Self::ProjectBudgetTooSmall { .. } => {
                "cache write skipped because payload exceeds project cache budget"
            }
            Self::NamespaceBudgetTooSmall { .. } => {
                "cache write skipped because payload exceeds namespace cache budget"
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredArtifactBytes {
    pub key: ArtifactKey,
    pub meta: ArtifactMeta,
    pub codec: PayloadCodec,
    pub compression: CompressionKind,
    pub payload: Vec<u8>,
}

pub struct DiskArtifactStore {
    root: PathBuf,
    objects: PathBuf,
    connection: Connection,
    options: DiskArtifactStoreOptions,
    policy: DiskArtifactStorePolicy,
    telemetry: RefCell<CacheTelemetry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct EvictionCandidate {
    key: ArtifactKey,
    payload_size: u64,
    priority: CachePriority,
    last_used_at: u64,
}

impl DiskArtifactStore {
    pub fn open(root: impl AsRef<Path>, options: DiskArtifactStoreOptions) -> CacheResult<Self> {
        Self::open_with_policy(root, options, DiskArtifactStorePolicy::default())
    }

    pub fn open_with_policy(
        root: impl AsRef<Path>,
        options: DiskArtifactStoreOptions,
        policy: DiskArtifactStorePolicy,
    ) -> CacheResult<Self> {
        let version_root = root.as_ref().join("v1");
        let objects = version_root.join("objects");
        fs::create_dir_all(&objects)?;
        let connection = Connection::open(version_root.join("cache.sqlite"))?;
        connection.busy_timeout(policy.busy_timeout)?;
        let store = Self {
            root: version_root,
            objects,
            connection,
            options,
            policy,
            telemetry: RefCell::new(CacheTelemetry::default()),
        };
        store.initialize_connection()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn telemetry(&self) -> CacheTelemetry {
        self.telemetry.borrow().clone()
    }

    pub fn record_compute_time(&self, key: &ArtifactKey, duration: Duration) {
        self.telemetry
            .borrow_mut()
            .record_compute_time(key, duration);
    }

    pub fn record_read_miss(&self, key: &ArtifactKey) {
        self.telemetry.borrow_mut().record_miss(key);
    }

    pub fn put_bytes(&mut self, artifact: DiskArtifactBytes) -> CacheResult<ArtifactMeta> {
        match self.put_bytes_with_report(artifact)?.status {
            DiskPutStatus::Stored(meta) => Ok(meta),
            DiskPutStatus::Skipped(reason) => {
                Err(CacheError::Unavailable(reason.message().to_owned()))
            }
        }
    }

    pub fn put_bytes_with_report(
        &mut self,
        artifact: DiskArtifactBytes,
    ) -> CacheResult<DiskPutReport> {
        self.validate_write_meta(&artifact.meta)?;
        let actual_payload_bytes = u64::try_from(artifact.payload.len()).map_err(|_| {
            CacheError::InvalidEnvelope("uncompressed payload is too large".to_owned())
        })?;
        if let Some(max_payload_bytes) = self.policy.budget.max_payload_bytes
            && actual_payload_bytes > max_payload_bytes
        {
            self.telemetry
                .borrow_mut()
                .record_skipped_write(&artifact.key);
            return Ok(DiskPutReport {
                key: artifact.key,
                status: DiskPutStatus::Skipped(DiskWriteSkipReason::PayloadTooLarge {
                    max_payload_bytes,
                    actual_payload_bytes,
                }),
            });
        }
        let stored_payload = compress_payload(&artifact.payload, artifact.compression)?;
        let payload_hash = hash_payload(&stored_payload);
        let stored_len = u64::try_from(stored_payload.len())
            .map_err(|_| CacheError::InvalidEnvelope("stored payload is too large".to_owned()))?;
        if let Some(reason) = self.budget_skip_reason(&artifact.key, stored_len) {
            self.telemetry
                .borrow_mut()
                .record_skipped_write(&artifact.key);
            return Ok(DiskPutReport {
                key: artifact.key,
                status: DiskPutStatus::Skipped(reason),
            });
        }
        let header = ArtifactEnvelopeHeader {
            magic: *b"APLCACHE",
            cache_schema_version: artifact.meta.cache_schema_version,
            compiler_version: artifact.meta.compiler_version.clone(),
            key: artifact.key.clone(),
            fingerprint: artifact.meta.fingerprint,
            codec: artifact.codec,
            compression: artifact.compression,
            uncompressed_len: actual_payload_bytes,
            stored_len,
            payload_hash,
        };
        self.write_object(&payload_hash, &header, &stored_payload)?;

        let meta = artifact.meta.with_payload(payload_hash, stored_len);
        let tx = self.write_transaction("write artifact metadata")?;
        if let Err(error) = upsert_artifact(&tx, &artifact.key, &meta)
            .and_then(|()| replace_dependencies(&tx, &artifact.key, &meta.dependencies))
            .and_then(|()| tx.commit().map_err(Into::into))
        {
            if is_lock_contention(&error) {
                return Err(cache_unavailable("write artifact metadata", error));
            }
            return Err(error);
        }
        self.enforce_budgets_after_write(&artifact.key)?;
        self.telemetry
            .borrow_mut()
            .record_compressed_bytes(&artifact.key, stored_len);
        Ok(DiskPutReport {
            key: artifact.key,
            status: DiskPutStatus::Stored(meta),
        })
    }

    pub fn put_metadata(
        &mut self,
        key: ArtifactKey,
        meta: ArtifactMeta,
    ) -> CacheResult<ArtifactMeta> {
        self.validate_write_meta(&meta)?;
        if meta.payload_hash.is_some() || meta.payload_size.is_some() {
            return Err(CacheError::Integrity(format!(
                "artifact {key} metadata-only write cannot include payload fields"
            )));
        }
        let tx = self.write_transaction("write artifact metadata")?;
        if let Err(error) = upsert_artifact(&tx, &key, &meta)
            .and_then(|()| replace_dependencies(&tx, &key, &meta.dependencies))
            .and_then(|()| tx.commit().map_err(Into::into))
        {
            if is_lock_contention(&error) {
                return Err(cache_unavailable("write artifact metadata", error));
            }
            return Err(error);
        }
        Ok(meta)
    }

    pub fn get_bytes(
        &mut self,
        options: &DiskReadOptions,
    ) -> CacheResult<Option<StoredArtifactBytes>> {
        let Some(meta) = self.meta(&options.key)? else {
            self.telemetry.borrow_mut().record_miss(&options.key);
            return Ok(None);
        };
        self.validate_read_meta(options, &meta)?;
        let Some(payload_hash) = meta.payload_hash else {
            self.telemetry.borrow_mut().record_miss(&options.key);
            return Ok(None);
        };
        let (header, stored_payload) = match self.read_object(&payload_hash) {
            Ok(object) => object,
            Err(error) if is_object_cache_miss(&error) => {
                self.remove_corrupt_metadata(&options.key);
                self.telemetry.borrow_mut().record_miss(&options.key);
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if let Err(error) = validate_stored_object(options, &header, &stored_payload, &payload_hash)
        {
            self.remove_corrupt_metadata(&options.key);
            if is_object_cache_miss(&error) {
                self.telemetry.borrow_mut().record_miss(&options.key);
                return Ok(None);
            }
            return Err(error);
        }
        let payload = match decompress_payload(&stored_payload, header.compression) {
            Ok(payload) => payload,
            Err(error) if is_object_cache_miss(&error) => {
                self.remove_corrupt_metadata(&options.key);
                self.telemetry.borrow_mut().record_miss(&options.key);
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        if u64::try_from(payload.len()).ok() != Some(header.uncompressed_len) {
            self.remove_corrupt_metadata(&options.key);
            self.telemetry.borrow_mut().record_miss(&options.key);
            return Ok(None);
        }
        if let Err(error) = self.touch(&options.key) {
            if !is_lock_contention(&error) {
                return Err(error);
            }
        }
        self.telemetry.borrow_mut().record_hit(&options.key);
        Ok(Some(StoredArtifactBytes {
            key: options.key.clone(),
            meta,
            codec: header.codec,
            compression: header.compression,
            payload,
        }))
    }

    pub fn gc_unreachable_objects(&self) -> CacheResult<Vec<PathBuf>> {
        let mut reachable = self
            .connection
            .prepare("SELECT payload_hash FROM artifacts WHERE payload_hash IS NOT NULL")?
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        reachable.sort();

        let mut removed = Vec::new();
        if !self.objects.exists() {
            return Ok(removed);
        }
        for shard in fs::read_dir(&self.objects)? {
            let shard = shard?;
            if !shard.file_type()?.is_dir() {
                continue;
            }
            for object in fs::read_dir(shard.path())? {
                let object = object?;
                if !object.file_type()?.is_file() {
                    continue;
                }
                let Some(name) = object.file_name().to_str().map(str::to_owned) else {
                    continue;
                };
                if is_temp_object_name(&name) {
                    if self.temp_object_is_stale(&object.path())
                        && remove_gc_candidate(&object.path())
                    {
                        removed.push(object.path());
                    }
                    continue;
                }
                let Some(hash) = name.strip_suffix(".bin") else {
                    continue;
                };
                if reachable
                    .binary_search_by(|candidate| candidate.as_str().cmp(hash))
                    .is_err()
                    && remove_gc_candidate(&object.path())
                {
                    removed.push(object.path());
                }
            }
        }
        Ok(removed)
    }

    fn initialize_connection(&self) -> CacheResult<()> {
        self.connection.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS artifacts(
              key TEXT PRIMARY KEY,
              namespace TEXT NOT NULL,
              kind TEXT NOT NULL,
              unit TEXT NOT NULL,
              fingerprint BLOB NOT NULL,
              payload_hash TEXT,
              payload_size INTEGER,
              compiler_version TEXT NOT NULL,
              std_version TEXT,
              options_hash TEXT,
              cache_schema_version INTEGER NOT NULL,
              revision INTEGER NOT NULL,
              created_at INTEGER NOT NULL,
              last_used_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS dependencies(
              artifact_key TEXT NOT NULL,
              depends_on_key TEXT NOT NULL,
              PRIMARY KEY (artifact_key, depends_on_key)
            );
            CREATE TABLE IF NOT EXISTS reverse_dependencies(
              dependency_key TEXT NOT NULL,
              dependent_key TEXT NOT NULL,
              PRIMARY KEY (dependency_key, dependent_key)
            );
            ",
        )?;
        self.migrate_nullable_payload_columns()?;
        Ok(())
    }

    fn migrate_nullable_payload_columns(&self) -> CacheResult<()> {
        let mut statement = self.connection.prepare("PRAGMA table_info(artifacts)")?;
        let columns = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, u32>(3)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let payload_columns_are_not_null = columns.iter().any(|(name, not_null)| {
            matches!(name.as_str(), "payload_hash" | "payload_size") && *not_null != 0
        });
        if !payload_columns_are_not_null {
            return Ok(());
        }
        self.connection.execute_batch(
            "
            ALTER TABLE artifacts RENAME TO artifacts_old_payload_required;
            CREATE TABLE artifacts(
              key TEXT PRIMARY KEY,
              namespace TEXT NOT NULL,
              kind TEXT NOT NULL,
              unit TEXT NOT NULL,
              fingerprint BLOB NOT NULL,
              payload_hash TEXT,
              payload_size INTEGER,
              compiler_version TEXT NOT NULL,
              std_version TEXT,
              options_hash TEXT,
              cache_schema_version INTEGER NOT NULL,
              revision INTEGER NOT NULL,
              created_at INTEGER NOT NULL,
              last_used_at INTEGER NOT NULL
            );
            INSERT INTO artifacts(
              key, namespace, kind, unit, fingerprint, payload_hash, payload_size,
              compiler_version, std_version, options_hash, cache_schema_version,
              revision, created_at, last_used_at
            )
            SELECT
              key, namespace, kind, unit, fingerprint, payload_hash, payload_size,
              compiler_version, std_version, options_hash, cache_schema_version,
              revision, created_at, last_used_at
            FROM artifacts_old_payload_required;
            DROP TABLE artifacts_old_payload_required;
            ",
        )?;
        Ok(())
    }

    fn validate_write_meta(&self, meta: &ArtifactMeta) -> CacheResult<()> {
        if meta.compiler_version != self.options.compiler_version {
            return Err(CacheError::Integrity(format!(
                "artifact compiler version {} does not match store compiler version {}",
                meta.compiler_version, self.options.compiler_version
            )));
        }
        if meta.cache_schema_version != self.options.cache_schema_version {
            return Err(CacheError::Integrity(format!(
                "artifact schema version {} does not match store schema version {}",
                meta.cache_schema_version, self.options.cache_schema_version
            )));
        }
        Ok(())
    }

    fn validate_read_meta(
        &self,
        options: &DiskReadOptions,
        meta: &ArtifactMeta,
    ) -> CacheResult<()> {
        if meta.fingerprint != options.fingerprint {
            return Err(CacheError::Integrity(format!(
                "artifact {} fingerprint mismatch",
                options.key
            )));
        }
        if meta.compiler_version != options.compiler_version {
            return Err(CacheError::Integrity(format!(
                "artifact {} compiler version mismatch",
                options.key
            )));
        }
        if meta.cache_schema_version != options.cache_schema_version {
            return Err(CacheError::Integrity(format!(
                "artifact {} schema version mismatch",
                options.key
            )));
        }
        Ok(())
    }

    fn write_object(
        &self,
        payload_hash: &ContentHash,
        header: &ArtifactEnvelopeHeader,
        stored_payload: &[u8],
    ) -> CacheResult<()> {
        let object_path = self.object_path(payload_hash);
        if object_path.exists() {
            self.validate_existing_object(payload_hash, header, stored_payload)?;
            return Ok(());
        }
        let parent = object_path.parent().ok_or_else(|| {
            CacheError::Io(std::io::Error::other("object path does not have parent"))
        })?;
        fs::create_dir_all(parent)?;
        let tmp_path = self.unique_temp_object_path(parent, payload_hash);
        let header_bytes = header.encode()?;
        let header_len = u32::try_from(header_bytes.len()).map_err(|_| {
            CacheError::InvalidEnvelope("cache envelope header is too large".to_owned())
        })?;
        {
            let mut tmp = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&tmp_path)?;
            tmp.write_all(&header_len.to_le_bytes())?;
            tmp.write_all(&header_bytes)?;
            tmp.write_all(stored_payload)?;
            tmp.sync_all()?;
        }
        match fs::hard_link(&tmp_path, &object_path) {
            Ok(()) => {
                fs::remove_file(&tmp_path)?;
                sync_directory(parent)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                remove_file_if_exists(&tmp_path)?;
                self.validate_existing_object(payload_hash, header, stored_payload)?;
            }
            Err(error) => {
                remove_file_if_exists(&tmp_path)?;
                return Err(error.into());
            }
        }
        Ok(())
    }

    fn validate_existing_object(
        &self,
        payload_hash: &ContentHash,
        expected_header: &ArtifactEnvelopeHeader,
        expected_stored_payload: &[u8],
    ) -> CacheResult<()> {
        let (header, stored_payload) = self.read_object(payload_hash)?;
        if &header != expected_header || stored_payload != expected_stored_payload {
            if stored_payload == expected_stored_payload {
                return Err(CacheError::Unavailable(format!(
                    "existing cache object {} has the same payload but a different envelope",
                    payload_hash
                )));
            }
            return Err(CacheError::Integrity(format!(
                "existing cache object {} does not match the artifact being published",
                payload_hash
            )));
        }
        Ok(())
    }

    fn unique_temp_object_path(&self, parent: &Path, payload_hash: &ContentHash) -> PathBuf {
        static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        parent.join(format!(
            ".{}-{}-{}-{}.tmp",
            payload_hash,
            std::process::id(),
            now_unix_seconds().unwrap_or(0),
            counter
        ))
    }

    fn read_object(
        &self,
        payload_hash: &ContentHash,
    ) -> CacheResult<(ArtifactEnvelopeHeader, Vec<u8>)> {
        let mut file = File::open(self.object_path(payload_hash))?;
        let mut len_bytes = [0u8; 4];
        file.read_exact(&mut len_bytes)?;
        let header_len = usize::try_from(u32::from_le_bytes(len_bytes)).map_err(|_| {
            CacheError::InvalidEnvelope("header length cannot fit usize".to_owned())
        })?;
        let mut header_bytes = vec![0u8; header_len];
        file.read_exact(&mut header_bytes)?;
        let header = ArtifactEnvelopeHeader::decode(&header_bytes)?;
        let mut payload = Vec::new();
        file.read_to_end(&mut payload)?;
        Ok((header, payload))
    }

    fn object_path(&self, payload_hash: &ContentHash) -> PathBuf {
        let hash = payload_hash.to_string();
        self.objects.join(&hash[..2]).join(format!("{hash}.bin"))
    }

    fn touch(&self, key: &ArtifactKey) -> CacheResult<()> {
        self.connection
            .execute(
                "UPDATE artifacts SET last_used_at = ?1 WHERE key = ?2",
                params![now_unix_seconds()?, encode_artifact_key(key)?],
            )
            .map_err(|error| {
                if is_sqlite_lock_contention(&error) {
                    cache_unavailable("touch artifact metadata", error.into())
                } else {
                    error.into()
                }
            })?;
        Ok(())
    }

    fn roots_for(&self, selector: InvalidationSelector) -> CacheResult<Vec<ArtifactKey>> {
        match selector {
            InvalidationSelector::Exact(key) => Ok(vec![key]),
            InvalidationSelector::Roots(keys) => Ok(keys),
            InvalidationSelector::Namespace(namespace) => {
                let mut statement = self.connection.prepare(
                    "SELECT namespace, kind, unit FROM artifacts WHERE namespace = ?1 ORDER BY key",
                )?;
                read_keys(statement.query_map(params![namespace.as_str()], read_key_row)?)
            }
            InvalidationSelector::Kind { namespace, kind } => {
                let mut statement = self.connection.prepare(
                    "SELECT namespace, kind, unit FROM artifacts WHERE namespace = ?1 AND kind = ?2 ORDER BY key",
                )?;
                read_keys(
                    statement
                        .query_map(params![namespace.as_str(), kind.as_str()], read_key_row)?,
                )
            }
        }
    }

    fn invalidation_closure(&self, roots: &[ArtifactKey]) -> CacheResult<Vec<ArtifactKey>> {
        let mut invalidated = crate::InvalidationSet::new();
        let mut queue = std::collections::VecDeque::from(roots.to_vec());
        while let Some(key) = queue.pop_front() {
            if !invalidated.insert(key.clone()) {
                continue;
            }
            let mut statement = self.connection.prepare(
                "SELECT dependent_key FROM reverse_dependencies WHERE dependency_key = ?1 ORDER BY dependent_key",
            )?;
            let dependents = statement
                .query_map(params![encode_artifact_key(&key)?], |row| {
                    row.get::<_, String>(0)
                })?
                .collect::<Result<Vec<_>, _>>()?;
            for dependent in dependents {
                queue.push_back(decode_artifact_key(&dependent)?);
            }
        }
        Ok(invalidated.into_keys())
    }

    fn remove_rows(&mut self, keys: &[ArtifactKey]) -> CacheResult<()> {
        let tx = self.write_transaction("remove artifact metadata")?;
        for key in keys {
            tx.execute(
                "DELETE FROM artifacts WHERE key = ?1",
                params![encode_artifact_key(key)?],
            )?;
            tx.execute(
                "DELETE FROM dependencies WHERE artifact_key = ?1 OR depends_on_key = ?1",
                params![encode_artifact_key(key)?],
            )?;
            tx.execute(
                "DELETE FROM reverse_dependencies WHERE dependency_key = ?1 OR dependent_key = ?1",
                params![encode_artifact_key(key)?],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn write_transaction(&mut self, context: &'static str) -> CacheResult<Transaction<'_>> {
        self.connection.transaction().map_err(|error| {
            if is_sqlite_lock_contention(&error) {
                cache_unavailable(context, error.into())
            } else {
                error.into()
            }
        })
    }

    fn remove_corrupt_metadata(&mut self, key: &ArtifactKey) {
        let _ = self.remove_rows(std::slice::from_ref(key));
    }

    fn budget_skip_reason(
        &self,
        key: &ArtifactKey,
        actual_payload_bytes: u64,
    ) -> Option<DiskWriteSkipReason> {
        if let Some(max_project_bytes) = self.policy.budget.max_project_bytes
            && actual_payload_bytes > max_project_bytes
        {
            return Some(DiskWriteSkipReason::ProjectBudgetTooSmall {
                max_project_bytes,
                actual_payload_bytes,
            });
        }
        if let Some(max_namespace_bytes) =
            self.policy.budget.namespace_budget(key.namespace.as_str())
            && actual_payload_bytes > max_namespace_bytes
        {
            return Some(DiskWriteSkipReason::NamespaceBudgetTooSmall {
                namespace: key.namespace.as_str().to_owned(),
                max_namespace_bytes,
                actual_payload_bytes,
            });
        }
        None
    }

    fn enforce_budgets_after_write(&mut self, key: &ArtifactKey) -> CacheResult<()> {
        if let Some(max_project_bytes) = self.policy.budget.max_project_bytes
            && let Err(error) = self.evict_to_budget(None, max_project_bytes)
            && !is_lock_contention(&error)
        {
            return Err(error);
        }
        if let Some(max_namespace_bytes) =
            self.policy.budget.namespace_budget(key.namespace.as_str())
            && let Err(error) =
                self.evict_to_budget(Some(key.namespace.as_str()), max_namespace_bytes)
            && !is_lock_contention(&error)
        {
            return Err(error);
        }
        Ok(())
    }

    fn evict_to_budget(&mut self, namespace: Option<&str>, max_bytes: u64) -> CacheResult<()> {
        let mut total = self.total_payload_size(namespace)?;
        if total <= max_bytes {
            return Ok(());
        }

        let mut candidates = self.eviction_candidates(namespace)?;
        candidates.sort_by(|left, right| {
            (
                left.priority,
                left.last_used_at,
                left.key.namespace.as_str(),
                left.key.kind.as_str(),
                left.key.unit.as_str(),
            )
                .cmp(&(
                    right.priority,
                    right.last_used_at,
                    right.key.namespace.as_str(),
                    right.key.kind.as_str(),
                    right.key.unit.as_str(),
                ))
        });

        let mut evicted = Vec::new();
        for candidate in candidates {
            if total <= max_bytes {
                break;
            }
            total = total.saturating_sub(candidate.payload_size);
            evicted.push(candidate.key);
        }
        if evicted.is_empty() {
            return Ok(());
        }
        self.remove_rows(&evicted)?;
        {
            let mut telemetry = self.telemetry.borrow_mut();
            for key in &evicted {
                telemetry.record_eviction(key);
            }
        }
        let _ = self.gc_unreachable_objects();
        Ok(())
    }

    fn total_payload_size(&self, namespace: Option<&str>) -> CacheResult<u64> {
        let total = match namespace {
            Some(namespace) => self.connection.query_row(
                "SELECT COALESCE(SUM(payload_size), 0) FROM artifacts WHERE namespace = ?1 AND payload_size IS NOT NULL",
                params![namespace],
                |row| row.get::<_, u64>(0),
            ),
            None => self.connection.query_row(
                "SELECT COALESCE(SUM(payload_size), 0) FROM artifacts WHERE payload_size IS NOT NULL",
                [],
                |row| row.get::<_, u64>(0),
            ),
        };
        total.map_err(Into::into)
    }

    fn eviction_candidates(&self, namespace: Option<&str>) -> CacheResult<Vec<EvictionCandidate>> {
        let rows = match namespace {
            Some(namespace) => {
                let mut statement = self.connection.prepare(
                    "SELECT namespace, kind, unit, payload_size, last_used_at FROM artifacts WHERE namespace = ?1 AND payload_size IS NOT NULL",
                )?;
                statement
                    .query_map(params![namespace], read_eviction_candidate_row)?
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => {
                let mut statement = self.connection.prepare(
                    "SELECT namespace, kind, unit, payload_size, last_used_at FROM artifacts WHERE payload_size IS NOT NULL",
                )?;
                statement
                    .query_map([], read_eviction_candidate_row)?
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(rows
            .into_iter()
            .map(|mut candidate| {
                candidate.priority = self.policy.budget.priority_for(&candidate.key);
                candidate
            })
            .collect())
    }

    fn temp_object_is_stale(&self, path: &Path) -> bool {
        let Ok(metadata) = fs::metadata(path) else {
            return false;
        };
        let Ok(modified) = metadata.modified() else {
            return false;
        };
        modified
            .elapsed()
            .is_ok_and(|age| age >= self.policy.stale_temp_file_age)
    }
}

impl ArtifactStore for DiskArtifactStore {
    fn contains(&self, key: &ArtifactKey) -> CacheResult<bool> {
        let count = self.connection.query_row(
            "SELECT COUNT(*) FROM artifacts WHERE key = ?1",
            params![encode_artifact_key(key)?],
            |row| row.get(0),
        );
        let count: u32 = match count {
            Ok(count) => count,
            Err(error) if is_sqlite_lock_contention(&error) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        Ok(count > 0)
    }

    fn meta(&self, key: &ArtifactKey) -> CacheResult<Option<ArtifactMeta>> {
        let row = match self
            .connection
            .query_row(
                "SELECT revision, fingerprint, payload_hash, payload_size, compiler_version, std_version, options_hash, cache_schema_version FROM artifacts WHERE key = ?1",
                params![encode_artifact_key(key)?],
                |row| {
                    Ok((
                        row.get::<_, u64>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<u64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, u32>(7)?,
                    ))
                },
            )
            .optional()
        {
            Ok(row) => row,
            Err(error) if is_sqlite_lock_contention(&error) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let Some((
            revision,
            fingerprint_blob,
            payload_hash,
            payload_size,
            compiler_version,
            std_version,
            options_hash,
            cache_schema_version,
        )) = row
        else {
            return Ok(None);
        };
        let fingerprint = fingerprint_from_blob(&fingerprint_blob)?;
        let mut meta = ArtifactMeta::new(
            ProjectRevision(revision),
            fingerprint,
            compiler_version,
            cache_schema_version,
        );
        if let Some(payload_hash) = payload_hash {
            let payload_hash = ContentHash::from_hex(&payload_hash).ok_or_else(|| {
                CacheError::Integrity(format!("artifact {key} metadata has invalid payload hash"))
            })?;
            let payload_size = payload_size.ok_or_else(|| {
                CacheError::Integrity(format!(
                    "artifact {key} metadata has payload hash without payload size"
                ))
            })?;
            meta = meta.with_payload(payload_hash, payload_size);
        } else if payload_size.is_some() {
            return Err(CacheError::Integrity(format!(
                "artifact {key} metadata has payload size without payload hash"
            )));
        }
        meta.std_version = std_version;
        meta.options_hash = options_hash;
        meta.dependencies = match self.dependencies_for(key) {
            Ok(dependencies) => dependencies,
            Err(error) if is_lock_contention(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        Ok(Some(meta))
    }

    fn remove(&mut self, key: &ArtifactKey) -> CacheResult<()> {
        self.remove_rows(std::slice::from_ref(key))
    }

    fn invalidate(&mut self, selector: InvalidationSelector) -> CacheResult<InvalidationReport> {
        let roots = match self.roots_for(selector) {
            Ok(roots) => roots,
            Err(error) if is_lock_contention(&error) => return Ok(InvalidationReport::default()),
            Err(error) => return Err(error),
        };
        let invalidated = match self.invalidation_closure(&roots) {
            Ok(invalidated) => invalidated,
            Err(error) if is_lock_contention(&error) => {
                return Ok(InvalidationReport {
                    roots,
                    invalidated: Vec::new(),
                });
            }
            Err(error) => return Err(error),
        };
        if let Err(error) = self.remove_rows(&invalidated) {
            if is_lock_contention(&error) {
                return Ok(InvalidationReport {
                    roots,
                    invalidated: Vec::new(),
                });
            }
            return Err(error);
        }
        Ok(InvalidationReport { roots, invalidated })
    }
}

impl DiskArtifactStore {
    fn dependencies_for(&self, key: &ArtifactKey) -> CacheResult<Vec<ArtifactKey>> {
        let mut statement = self.connection.prepare(
            "SELECT depends_on_key FROM dependencies WHERE artifact_key = ?1 ORDER BY depends_on_key",
        )?;
        let rows = statement.query_map(params![encode_artifact_key(key)?], |row| {
            row.get::<_, String>(0)
        })?;
        let mut dependencies = rows
            .map(|row| decode_artifact_key(&row?))
            .collect::<CacheResult<Vec<_>>>()?;
        dependencies.sort();
        dependencies.dedup();
        Ok(dependencies)
    }
}

fn upsert_artifact(
    tx: &Transaction<'_>,
    key: &ArtifactKey,
    meta: &ArtifactMeta,
) -> CacheResult<()> {
    if meta.payload_hash.is_some() != meta.payload_size.is_some() {
        return Err(CacheError::Integrity(format!(
            "artifact {key} metadata payload hash and size must be both present or both absent"
        )));
    }
    let now = now_unix_seconds()?;
    let payload_hash = meta.payload_hash.map(|hash| hash.to_string());
    tx.execute(
        "
        INSERT INTO artifacts(
          key, namespace, kind, unit, fingerprint, payload_hash, payload_size,
          compiler_version, std_version, options_hash, cache_schema_version,
          revision, created_at, last_used_at
        )
        VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?13)
        ON CONFLICT(key) DO UPDATE SET
          fingerprint = excluded.fingerprint,
          payload_hash = excluded.payload_hash,
          payload_size = excluded.payload_size,
          compiler_version = excluded.compiler_version,
          std_version = excluded.std_version,
          options_hash = excluded.options_hash,
          cache_schema_version = excluded.cache_schema_version,
          revision = excluded.revision,
          last_used_at = excluded.last_used_at
        ",
        params![
            encode_artifact_key(key)?,
            key.namespace.as_str(),
            key.kind.as_str(),
            key.unit.as_str(),
            meta.fingerprint.as_bytes().as_slice(),
            payload_hash.as_deref(),
            meta.payload_size,
            meta.compiler_version.as_str(),
            meta.std_version.as_deref(),
            meta.options_hash.as_deref(),
            meta.cache_schema_version,
            meta.revision.0,
            now
        ],
    )?;
    Ok(())
}

fn replace_dependencies(
    tx: &Transaction<'_>,
    artifact: &ArtifactKey,
    dependencies: &[ArtifactKey],
) -> CacheResult<()> {
    tx.execute(
        "DELETE FROM dependencies WHERE artifact_key = ?1",
        params![encode_artifact_key(artifact)?],
    )?;
    tx.execute(
        "DELETE FROM reverse_dependencies WHERE dependent_key = ?1",
        params![encode_artifact_key(artifact)?],
    )?;
    for dependency in dependencies {
        tx.execute(
            "INSERT OR REPLACE INTO dependencies(artifact_key, depends_on_key) VALUES(?1, ?2)",
            params![
                encode_artifact_key(artifact)?,
                encode_artifact_key(dependency)?
            ],
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO reverse_dependencies(dependency_key, dependent_key) VALUES(?1, ?2)",
            params![encode_artifact_key(dependency)?, encode_artifact_key(artifact)?],
        )?;
    }
    Ok(())
}

fn read_key_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactKey> {
    Ok(ArtifactKey::new(
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
    ))
}

fn read_eviction_candidate_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvictionCandidate> {
    Ok(EvictionCandidate {
        key: ArtifactKey::new(
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ),
        payload_size: row.get::<_, u64>(3)?,
        priority: CachePriority::Normal,
        last_used_at: row.get::<_, u64>(4)?,
    })
}

fn read_keys(
    rows: impl Iterator<Item = rusqlite::Result<ArtifactKey>>,
) -> CacheResult<Vec<ArtifactKey>> {
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn validate_header(options: &DiskReadOptions, header: &ArtifactEnvelopeHeader) -> CacheResult<()> {
    if header.key != options.key {
        return Err(CacheError::Integrity(format!(
            "artifact {} envelope key mismatch",
            options.key
        )));
    }
    if header.fingerprint != options.fingerprint {
        return Err(CacheError::Integrity(format!(
            "artifact {} envelope fingerprint mismatch",
            options.key
        )));
    }
    if header.compiler_version != options.compiler_version {
        return Err(CacheError::Integrity(format!(
            "artifact {} envelope compiler version mismatch",
            options.key
        )));
    }
    if header.cache_schema_version != options.cache_schema_version {
        return Err(CacheError::Integrity(format!(
            "artifact {} envelope schema version mismatch",
            options.key
        )));
    }
    Ok(())
}

fn validate_stored_object(
    options: &DiskReadOptions,
    header: &ArtifactEnvelopeHeader,
    stored_payload: &[u8],
    payload_hash: &ContentHash,
) -> CacheResult<()> {
    validate_header(options, header)?;
    if header.payload_hash != *payload_hash {
        return Err(CacheError::Integrity(format!(
            "artifact {} metadata hash and envelope hash differ",
            options.key
        )));
    }
    let actual_hash = hash_payload(stored_payload);
    if actual_hash != *payload_hash {
        return Err(CacheError::Integrity(format!(
            "artifact {} stored payload hash mismatch",
            options.key
        )));
    }
    if u64::try_from(stored_payload.len()).ok() != Some(header.stored_len) {
        return Err(CacheError::Integrity(format!(
            "artifact {} stored payload length mismatch",
            options.key
        )));
    }
    Ok(())
}

fn hash_payload(payload: &[u8]) -> ContentHash {
    ContentHash::new(*blake3::hash(payload).as_bytes())
}

fn now_unix_seconds() -> CacheResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| {
            CacheError::Integrity(format!("system clock is before UNIX epoch: {error}"))
        })
}

fn sync_directory(path: &Path) -> CacheResult<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> CacheResult<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn remove_gc_candidate(path: &Path) -> bool {
    fs::remove_file(path).is_ok()
}

fn is_temp_object_name(name: &str) -> bool {
    name.starts_with('.') && name.ends_with(".tmp")
}

fn is_sqlite_lock_contention(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(failure, _)
            if matches!(failure.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

fn is_lock_contention(error: &CacheError) -> bool {
    match error {
        CacheError::Sqlite(error) => is_sqlite_lock_contention(error),
        CacheError::Unavailable(_) => true,
        _ => false,
    }
}

fn cache_unavailable(context: &str, error: CacheError) -> CacheError {
    CacheError::Unavailable(format!("{context}: {error}"))
}

fn is_object_cache_miss(error: &CacheError) -> bool {
    matches!(
        error,
        CacheError::Io(_)
            | CacheError::InvalidEnvelope(_)
            | CacheError::Integrity(_)
            | CacheError::Unavailable(_)
    )
}

fn fingerprint_from_blob(blob: &[u8]) -> CacheResult<ArtifactFingerprint> {
    if blob.len() != 32 {
        return Err(CacheError::Integrity(format!(
            "artifact metadata fingerprint has invalid length {}",
            blob.len()
        )));
    }
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(blob);
    Ok(ArtifactFingerprint::new(bytes))
}

fn encode_artifact_key(key: &ArtifactKey) -> CacheResult<String> {
    let mut bytes = Vec::new();
    push_key_part(&mut bytes, key.namespace.as_str())?;
    push_key_part(&mut bytes, key.kind.as_str())?;
    push_key_part(&mut bytes, key.unit.as_str())?;
    Ok(hex_encode(&bytes))
}

fn decode_artifact_key(value: &str) -> CacheResult<ArtifactKey> {
    let bytes = hex_decode(value)?;
    let mut cursor = KeyCursor::new(&bytes);
    let namespace = cursor.read_part()?;
    let kind = cursor.read_part()?;
    let unit = cursor.read_part()?;
    cursor.finish()?;
    Ok(ArtifactKey::new(namespace, kind, unit))
}

fn push_key_part(bytes: &mut Vec<u8>, value: &str) -> CacheResult<()> {
    let len = u32::try_from(value.len()).map_err(|_| {
        CacheError::Integrity("artifact key part is too large for disk key encoding".to_owned())
    })?;
    bytes.extend_from_slice(&len.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hex_decode(value: &str) -> CacheResult<Vec<u8>> {
    if value.len() % 2 != 0 {
        return Err(CacheError::Integrity(
            "artifact key encoding has odd hex length".to_owned(),
        ));
    }
    let mut bytes = Vec::with_capacity(value.len() / 2);
    for index in (0..value.len()).step_by(2) {
        let byte = u8::from_str_radix(&value[index..index + 2], 16).map_err(|error| {
            CacheError::Integrity(format!("artifact key encoding is not hex: {error}"))
        })?;
        bytes.push(byte);
    }
    Ok(bytes)
}

struct KeyCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> KeyCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_part(&mut self) -> CacheResult<String> {
        let len_bytes = self.read_array::<4>()?;
        let len = usize::try_from(u32::from_le_bytes(len_bytes)).map_err(|_| {
            CacheError::Integrity("artifact key part length cannot fit usize".to_owned())
        })?;
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| CacheError::Integrity("artifact key part offset overflow".to_owned()))?;
        let Some(part) = self.bytes.get(self.offset..end) else {
            return Err(CacheError::Integrity(
                "artifact key encoding ended early".to_owned(),
            ));
        };
        self.offset = end;
        String::from_utf8(part.to_vec()).map_err(|error| {
            CacheError::Integrity(format!("artifact key part is not utf8: {error}"))
        })
    }

    fn read_array<const N: usize>(&mut self) -> CacheResult<[u8; N]> {
        let end = self
            .offset
            .checked_add(N)
            .ok_or_else(|| CacheError::Integrity("artifact key offset overflow".to_owned()))?;
        let Some(slice) = self.bytes.get(self.offset..end) else {
            return Err(CacheError::Integrity(
                "artifact key encoding ended early".to_owned(),
            ));
        };
        self.offset = end;
        let mut result = [0u8; N];
        result.copy_from_slice(slice);
        Ok(result)
    }

    fn finish(self) -> CacheResult<()> {
        if self.offset == self.bytes.len() {
            return Ok(());
        }
        Err(CacheError::Integrity(
            "artifact key encoding has trailing bytes".to_owned(),
        ))
    }
}
