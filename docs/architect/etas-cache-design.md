# Etas Cache Design

Status: `Draft`

Owner: `Architect`

Last updated: `2026-06-10`

## 1. Purpose

`etas_cache` is the shared artifact cache and dependency-tracking foundation for
compiler, IDE, tooling, and future build services. It belongs in `etas-core`
because frontend, optimizing, IDE/watch mode, future build daemons, and selected
interpreter/runtime planning tools may all need the same storage, dependency,
and invalidation primitives.

`etas_cache` is not a frontend crate. It does not understand Etas syntax, HIR,
types, effects, standard-library semantics, interpreter behavior, LSP, or CLI
policy. It stores and invalidates caller-defined artifacts under caller-defined
namespaces.

## 2. Boundary

`etas_cache` owns:

- artifact keys and cache namespaces;
- revision and fingerprint primitives;
- dependency edges between artifact keys;
- invalidation sets and invalidation selectors;
- cache metadata;
- memory artifact store;
- disk artifact store;
- SQLite metadata index;
- content-addressed binary blob object store;
- cache policy and eviction primitives.

`etas_cache` does not own:

- parser, AST, HIR, type, or effect semantics;
- frontend pass ordering;
- frontend artifact kinds such as `HirModule` or `TypeFacts`;
- optimizing artifact kinds such as `FirModule` or `AirModule`;
- IDE indexes, hover, completion, or LSP values;
- runtime checkpoints or durable execution state.

The split is:

```text
etas_cache
  generic key/fingerprint/store/dependency primitives

etas_frontend
  frontend artifact kinds, semantic dependencies, invalidation meaning,
  session orchestration, snapshot/delta contracts

etas_optimizing / etas_intel / other callers
  their own artifact kinds, dependency meaning, and recomputation logic
```

## 3. Crate Layout

Recommended layout:

```text
crates/etas_cache/
  src/
    lib.rs

    key.rs
    revision.rs
    fingerprint.rs
    dependency.rs
    invalidation.rs
    policy.rs
    meta.rs

    store/
      mod.rs
      trait.rs
      memory.rs
      disk.rs
      object_store.rs
      sqlite_index.rs

    serialize/
      mod.rs
      envelope.rs
      codec.rs
      compression.rs
```

The intended implementation is a layered cache:

```text
MemoryArtifactStore
  hot typed artifacts for IDE/watch/daemon sessions

DiskArtifactStore
  SQLite metadata index
  + content-addressed binary blob files
```

The disk store should not be pure JSON and should not store all artifact payloads
as SQLite blobs. SQLite owns indexes, dependency edges, version metadata, and
transactions. Large artifact payloads live in content-addressed binary files.

## 4. Core Types

Recommended public shape:

```rust
pub struct CacheNamespace(pub String);
pub struct ArtifactKindKey(pub String);
pub struct ArtifactUnitKey(pub String);
pub struct ProjectRevision(pub u64);
pub struct ArtifactFingerprint(pub [u8; 32]);
pub struct ContentHash(pub [u8; 32]);

pub struct ArtifactKey {
    pub namespace: CacheNamespace,
    pub kind: ArtifactKindKey,
    pub unit: ArtifactUnitKey,
}

pub struct ArtifactMeta {
    pub revision: ProjectRevision,
    pub fingerprint: ArtifactFingerprint,
    pub payload_hash: Option<ContentHash>,
    pub payload_size: Option<u64>,
    pub dependencies: Vec<ArtifactKey>,
    pub compiler_version: String,
    pub cache_schema_version: u32,
}

pub struct CachedArtifact<T> {
    pub key: ArtifactKey,
    pub meta: ArtifactMeta,
    pub value: T,
}
```

`ArtifactKindKey` is caller-defined. For example, `etas_frontend` may map its
own enum to keys such as `frontend.parsed_source`, `frontend.hir_module`, or
`frontend.type_facts`. `etas_cache` must not contain those frontend enum
variants directly.

Example caller-defined keys:

```text
frontend.parsed_source:<SourceId>
frontend.hir_module:<ModuleId>
frontend.type_facts:<BodyId>
optimizing.fir_module:<ModuleId>
optimizing.air_module:<ModuleId>
ide.workspace_symbol_index:<ProjectId>
interpreter.plan:<EntryPoint>
```

`etas_cache` treats all of these uniformly as namespaced artifact keys.

## 5. Store Interface

Recommended trait:

```rust
pub trait ArtifactStore {
    fn contains(&self, key: &ArtifactKey) -> bool;
    fn meta(&self, key: &ArtifactKey) -> Option<ArtifactMeta>;
    fn remove(&mut self, key: &ArtifactKey);
    fn invalidate(&mut self, selector: InvalidationSelector) -> InvalidationReport;
}

pub trait TypedArtifactStore: ArtifactStore {
    fn get<T: Clone + 'static>(&self, key: &ArtifactKey) -> Option<CachedArtifact<T>>;
    fn put<T: Clone + 'static>(&mut self, artifact: CachedArtifact<T>);
}
```

The exact Rust API can evolve, but the conceptual boundary must remain:

- storage operations are generic;
- semantic artifact construction belongs to the caller;
- cache misses must be recoverable by recomputing the artifact.

The memory store is typed and process-local. The disk store is serialized and
cross-process reusable. Callers that use disk cache must provide codecs for their
own artifact payloads.

## 6. Dependency Graph

`etas_cache` should provide a generic dependency graph:

```rust
pub struct ArtifactDependencyGraph;

impl ArtifactDependencyGraph {
    pub fn add_dependency(&mut self, artifact: ArtifactKey, depends_on: ArtifactKey);
    pub fn dependents_of(&self, key: &ArtifactKey) -> Vec<ArtifactKey>;
    pub fn invalidate_from(&self, roots: &[ArtifactKey]) -> InvalidationSet;
}
```

It does not decide what an edge means. `etas_frontend` decides that
`TypeFacts(body)` depends on `HirBody(body)`, `SignatureFacts(scope)`, and
`StdRegistry(version)`.

## 7. Frontend Usage

The frontend should use `etas_cache` like this:

```text
FrontendSession
  -> computes frontend ArtifactKey values
  -> asks ArtifactStore for reusable artifacts
  -> runs passes on cache miss or invalidation
  -> writes new artifacts and dependency metadata
  -> emits ProjectSemanticSnapshot and ProjectSemanticDelta
```

Example frontend-defined artifact keys:

```text
frontend.parsed_source:<SourceId>
frontend.module_index:<ProjectId>
frontend.import_graph:<ProjectId>
frontend.hir_module:<ModuleId>
frontend.type_facts:<BodyId>
frontend.effect_facts:<BodyId>
frontend.diagnostics:<SourceId>
```

These names are examples. The actual frontend key mapping belongs in
`etas_frontend`, not in `etas_cache`.

## 8. Other Caller Usage

The cache layer is generic. Frontend is only the first major user.

Future callers may use their own namespaces:

```text
etas_frontend
  frontend.parsed_source
  frontend.module_index
  frontend.hir_module
  frontend.type_facts
  frontend.effect_facts

etas_optimizing
  optimizing.fir_module
  optimizing.dataflow_facts
  optimizing.optimized_fir
  optimizing.air_module

etas_intel
  ide.workspace_symbol_index
  ide.semantic_token_index
  ide.definition_index

etas_interpreter
  interpreter.entry_plan
  interpreter.intrinsic_dispatch_plan
```

Each caller owns:

- artifact kind names;
- fingerprint computation;
- serialization/deserialization of payloads;
- semantic dependency edges;
- cache hit validation beyond generic version/fingerprint checks;
- recomputation on cache miss.

`etas_cache` owns:

- storage;
- dependency edge persistence;
- invalidation closure calculation;
- content hashing and payload integrity checks;
- memory/disk cache policy;
- garbage collection of unreachable disk objects.

Runtime checkpoint/resume records are explicitly not cache artifacts. They are
user-visible durable execution state with audit, consistency, and replay
requirements. `etas_cache` may cache derived indexes over checkpoint metadata in
the future, but not checkpoint bodies themselves.

## 9. Disk Store Architecture

Disk cache artifacts must be:

- versioned by compiler version and cache schema version;
- keyed by project, options, std registry version, source/dependency
  fingerprints, and artifact kind;
- safe to delete at any time;
- validated before use;
- never treated as the canonical source of program truth.

Recommended disk layout:

```text
.etas/cache/
  v1/
    cache.sqlite
    objects/
      ab/
        abcdef...bin
      cd/
        cdef12...bin
```

SQLite stores metadata and dependency edges. Blob files store serialized artifact
envelopes and payloads.

Recommended metadata schema:

```sql
artifacts(
  key TEXT PRIMARY KEY,
  namespace TEXT NOT NULL,
  kind TEXT NOT NULL,
  unit TEXT NOT NULL,
  fingerprint BLOB NOT NULL,
  payload_hash TEXT NOT NULL,
  payload_size INTEGER NOT NULL,
  compiler_version TEXT NOT NULL,
  std_version TEXT,
  options_hash TEXT,
  cache_schema_version INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  last_used_at INTEGER NOT NULL
);

dependencies(
  artifact_key TEXT NOT NULL,
  depends_on_key TEXT NOT NULL,
  PRIMARY KEY (artifact_key, depends_on_key)
);

reverse_dependencies(
  dependency_key TEXT NOT NULL,
  dependent_key TEXT NOT NULL,
  PRIMARY KEY (dependency_key, dependent_key)
);
```

Recommended payload envelope types:

```rust
pub enum PayloadCodec {
    Bincode2,
    Postcard,
}

pub enum CompressionKind {
    None,
    Zstd,
}

pub struct ArtifactEnvelope {
    pub meta: ArtifactMeta,
    pub codec: PayloadCodec,
    pub compression: CompressionKind,
    pub payload: Vec<u8>,
}
```

The on-disk blob format must be explicit and self-describing enough for
validation, while keeping artifact semantics outside `etas_cache`.

Recommended blob byte layout:

```text
u32 little-endian header_len
header_bytes
payload_bytes
```

`header_bytes` is a cache-owned binary header. It records how to validate and
decode `payload_bytes`; it is not the caller artifact itself.

Recommended header shape:

```rust
pub struct ArtifactEnvelopeHeader {
    pub magic: [u8; 8],              // b"APLCACHE"
    pub cache_schema_version: u32,
    pub compiler_version: String,
    pub key: ArtifactKey,
    pub fingerprint: ArtifactFingerprint,
    pub codec: PayloadCodec,
    pub compression: CompressionKind,
    pub uncompressed_len: u64,
    pub stored_len: u64,
    pub payload_hash: ContentHash,
}
```

`payload_bytes` contains the caller artifact after caller serialization and
optional compression. `payload_hash` hashes the stored payload bytes, not the
logical artifact value. This makes object addressing, deduplication, and
integrity checks independent from caller types.

Generation flow:

```text
caller artifact value
  -> caller codec serializes value into raw bytes
  -> etas_cache compresses raw bytes when configured
  -> etas_cache hashes stored payload bytes with blake3
  -> etas_cache builds ArtifactEnvelopeHeader
  -> etas_cache writes header_len + header_bytes + payload_bytes to a temp file
  -> etas_cache fsyncs and atomically renames temp file to objects/xx/hash.bin
  -> etas_cache commits SQLite metadata and dependency rows in one transaction
```

Parsing flow:

```text
lookup ArtifactKey in SQLite
  -> read objects/xx/hash.bin by payload_hash
  -> read header_len and header_bytes
  -> validate magic, schema version, compiler version, key, and fingerprint
  -> verify blake3(payload_bytes) == payload_hash
  -> decompress payload_bytes according to header.compression
  -> ask caller codec to deserialize the expected artifact type
```

The caller owns the payload codec for its artifact type. `etas_cache` owns the
envelope, compression, hashing, object path, SQLite metadata, dependency rows,
atomic write discipline, and validation sequence.

The object path is derived from the stored payload hash:

```text
objects/<first-two-hex>/<full-hash>.bin
```

Blob files are immutable once written. Rewriting an artifact with new content
creates a new object and updates SQLite metadata to point at the new hash.
Unreferenced old objects are removed only by cache GC.

Recommended implementation choices:

- payload hash: `blake3`;
- compression: `zstd`;
- binary codec: `bincode 2` or `postcard`;
- metadata/index: `sqlite`;
- payload storage: content-addressed object files.

The cache is internal and rebuildable, so the serialized payload format does not
need to be a stable public interchange format. `cache_schema_version`,
`compiler_version`, std registry version, options hash, and artifact
fingerprints determine whether a payload is still valid.

Write flow:

```text
serialize artifact payload
  -> compress
  -> hash compressed payload
  -> write objects/xx/hash.bin atomically
  -> upsert artifact metadata in SQLite transaction
  -> write dependency and reverse-dependency edges
```

Read flow:

```text
lookup metadata by ArtifactKey
  -> validate schema/compiler/std/options/fingerprint
  -> read object by payload_hash
  -> verify blake3 hash
  -> decompress
  -> deserialize with caller codec
```

Invalidation flow:

```text
root changed artifact keys
  -> query reverse_dependencies
  -> compute invalidation closure
  -> remove metadata rows
  -> leave unreferenced blob objects
  -> GC deletes unreachable objects later
```

Disk cache should store rebuildable compiler/tooling artifacts, not formatted UI
results such as hover markdown, completion ranking, or LSP diagnostics unless an
IDE caller explicitly defines those as rebuildable index artifacts in its own
namespace.

## 10. Cross-Process Visibility And Locking

`DiskArtifactStore` is a cross-process visible artifact cache. If one process
runs `etas watch` and writes valid artifacts under `.etas/cache/v1/`, another
process running `etas check`, `etas run`, or a future IDE-backed command may
open the same cache root and reuse those artifacts after validation.

This does not make `etas_cache` a live compiler session:

```text
MemoryArtifactStore
  process-local hot artifacts only

DiskArtifactStore
  cross-process visible rebuildable artifacts
```

The disk cache must be safe under concurrent readers and writers. It must never
require a single long-running owner process, and it must never corrupt the
project if two Etas processes operate on the same cache root.

SQLite connection setup must include:

- `PRAGMA journal_mode = WAL`;
- `PRAGMA foreign_keys = ON`;
- a bounded `busy_timeout`, recommended default `500ms` to `1000ms`;
- a cache schema version check before reading persisted artifacts.

`synchronous = NORMAL` is acceptable for a rebuildable cache when WAL is used.
`synchronous = FULL` may be offered as a conservative option, but frontend,
CLI, and IDE correctness must not depend on cache durability. The source files
and project manifest remain canonical.

Reader/writer behavior:

- multiple readers may read the SQLite index concurrently;
- SQLite allows only one writer at a time, so metadata write transactions must
  be short;
- parsing, type checking, effect checking, artifact serialization, compression,
  and hashing must happen outside SQLite write transactions;
- a writer transaction should only upsert metadata and dependency rows for
  already-materialized blob objects;
- readers must validate metadata, envelope, hash, schema, compiler version,
  options hash, std version, and caller fingerprints before using an artifact;
- if validation fails, the result is a cache miss or cache invalidation, not a
  compiler correctness failure.

Write publication order is mandatory:

```text
compute caller artifact
  -> serialize payload outside SQLite transaction
  -> compress payload outside SQLite transaction
  -> hash stored payload bytes
  -> write blob to unique temp file in the target shard directory
  -> fsync temp file
  -> atomically rename temp file to objects/xx/hash.bin
  -> fsync shard directory
  -> open short SQLite transaction
  -> upsert artifact metadata pointing at hash
  -> replace dependency and reverse-dependency rows
  -> commit transaction
```

This ordering guarantees that committed metadata does not point at a partially
written object. If a process crashes before the SQLite transaction commits, the
cache may contain an orphan blob; that is safe and later GC may remove it. If a
process crashes after the SQLite transaction commits, readers can validate the
complete blob by hash and envelope.

Concurrent object writes must be content-addressed and idempotent:

- object paths are derived only from `blake3(stored_payload_bytes)`;
- temp file names must include enough process-local uniqueness to avoid
  collisions, such as process id plus monotonic counter or random nonce;
- two writers producing the same payload may race to publish the same final
  object path;
- if the final object already exists, the losing writer must verify that the
  existing object's payload hash and envelope are valid for the stored bytes and
  then reuse it;
- a writer must not truncate or rewrite an existing final object;
- final blob files are immutable.

SQLite locking policy:

- lock waits must be bounded by `busy_timeout`;
- if a read cannot obtain the needed lock within the timeout, the caller should
  treat the disk cache as unavailable and recompute;
- if a write cannot obtain the writer lock within the timeout, the caller should
  skip persisting that artifact for this run and continue with in-memory
  artifacts;
- cache lock contention must not make `etas check` or `etas run` fail unless
  the user explicitly requested strict cache diagnostics;
- lock timeout events may be reported as debug/trace metadata, not ordinary
  frontend diagnostics.

Locked fallback behavior:

```text
read lock timeout
  -> report cache read miss to caller
  -> recompute artifact
  -> optionally try to store later

write lock timeout
  -> keep artifact in MemoryArtifactStore
  -> skip disk write for this artifact
  -> continue compilation

corrupt or missing blob
  -> remove or ignore metadata row when safe
  -> treat artifact as cache miss
  -> recompute from canonical inputs
```

GC must also be concurrency-aware:

- GC must not run inside long compiler transactions;
- GC must only delete final `objects/xx/<hash>.bin` files that are unreachable
  from committed SQLite metadata;
- GC must ignore temp files from other active processes unless they are older
  than a conservative stale-temp threshold;
- the stale-temp threshold must be configurable and default to a conservative
  duration, such as several hours;
- GC must tolerate another process creating or deleting the same object between
  scan and delete;
- GC failure must not fail compilation.

Required multi-process tests:

- two independent `DiskArtifactStore` instances can open the same root; one
  writes an artifact and the other reads it after validation;
- a reader can keep reading old metadata while another store writes a different
  artifact;
- concurrent writers publishing the same payload leave one valid final object
  and valid metadata;
- concurrent writers publishing different payloads serialize metadata writes
  without corrupting dependency rows;
- lock timeout during read is surfaced as cache miss/fallback rather than a
  compile failure;
- lock timeout during write keeps compilation successful and leaves the artifact
  available in memory;
- orphan blobs produced by a simulated crash-before-metadata are ignored by
  reads and later removed by GC;
- missing or corrupt blobs referenced by metadata are rejected by validation and
  treated as cache misses;
- GC does not remove active temp files or valid committed objects;
- frontend disk-backed incremental checks can reuse artifacts across fresh
  sessions while another process has the cache open.

## 11. Non-Goals

`etas_cache` must not become:

- a compiler session;
- a frontend pass manager;
- a persistent runtime checkpoint store;
- a database for user program state;
- an IDE index store.

Runtime checkpoint/resume state belongs to interpreter/runtime execution design.
IDE query indexes belong to `etas_intel`. Compiler artifact semantics belong to
`etas_frontend` and the semantic crates that produce those artifacts.
