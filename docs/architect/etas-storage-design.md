# Bounded Versioned Storage Design

Status: `Architecture accepted; implementation acceptance pending`

Owner: `Architect`

Last updated: `2026-09-10`

## 1. Scope And Language Boundary

This design addresses [core#12](https://github.com/etas-project/etas-core/issues/12)
and refines the storage contracts in [Host Design](etas-host-design.md).
Reuse `MemoryRegionRef`, `StoreRef`, `MemoryVersion`, `MemoryConflict`, memory
and session clients, and [Shared Execution Lifecycle](etas-execution-design.md).
No new crate, application session framework or common HIR/AIR representation
is required.

The [language SPEC](../../../etas/docs/design/03-agents-tools-prompts-memory.md#33-sessions-and-conversations)
now specifies bounded session history, prepared context publication and
reconciliation, and removes automatic summary generation from Session maintenance.
Section 7 implements that source contract; removing `SessionCompactor` and
`SummarizeWhen` no longer awaits a language-boundary decision. Retention and
storage compaction remain runtime/storage responsibilities, distinct from
generating a semantic summary.

The general Memory intent API in section 8 is a separate accepted architecture
contract. Its full source declarations/error rows are not yet specified by this
Session SPEC update and still require synchronization. Neither SPEC alignment
nor architecture acceptance is evidence of implementation acceptance.

| Owner | Responsibility |
|---|---|
| `etas_host::storage` | Shared version/condition contracts, limits, transaction evidence, receipt mechanisms and managed database execution |
| `etas_host::memory` | Store schemas, typed-memory protocol, queries and backend-specific memory statements |
| `etas_host::session` | Message append/deduplication, bounded history selection, revision fences, conditional context publication and configured retention/storage maintenance |
| `etas_host::value` | Shared bounded lossless stored-value codec; no HIR/AIR interpretation |
| `etas_host::execution` | Existing cancellation signals, operation registration, ownership and shutdown reporting |
| `etas_std` and execution engines | Source declarations and checked effects; type-directed request/result conversion; checkpoint evidence |
| EDK and applications | Schema, context selection, summarization/tokenization, retention decisions and retry/reconciliation policy |
| Operators | Backend configuration, resource ceilings, disk quotas, backup and deployment isolation |

`etas_cache` is not a durable state backend. A cache miss can recompute an
artifact; lost application state cannot be replaced by an empty/default value.
In-memory storage implements the same supported conditional-operation semantics
but explicitly offers no restart durability. No adapter silently falls back
from persistent storage to in-memory storage.

## 2. Version Identity And Conditional Writes

Keep `MemoryVersion` opaque to consumers. Its validated internal identity binds
a Store incarnation/generation and a Store-wide committed revision. Scope is
the configured backend namespace plus region and Store, not a path string or
per-process counter. Tokens are concurrency evidence, not access grants.

Required SQLite metadata consists of a durable Store generation and a checked
monotonic revision counter, separate from individual entry rows. Every successful
entry mutation allocates a revision and updates the entry in the same transaction.
Revisions are unique across entries within a generation; matching a token against
the addressed row consequently cannot accept a token from a different key.
Canonical key encoding is shared across lookup, mutation, cursor and receipt
fingerprinting; serialization failure is an error, never a Debug-string key.

- Opening another connection or restarting a process preserves generation and
  counter. Do not mint a generation on every client open.
- Deleting an entry does not remove/reset the Store counter. Recreating it gets
  a fresh revision even if its value is identical.
- Dropping/recreating a Store creates a fresh generation. Restoring or forking an
  older database requires an explicit, coordinated generation transition before
  accepting new writes. Ordinary reopen must not rotate a live shared Store.
- Counters never wrap. Enforce the actual backend integer range and fail before
  mutation when exhausted.
- Validate token format, size and scope. Do not accept legacy decimal tokens by
  guessing their generation or substitute an unconditional write on mismatch.

Generation transitions are a managed restore/provisioning contract. Host cannot
detect every out-of-band filesystem copy or rollback of an entire database;
operators must not present such a copy as an unchanged live Store incarnation.

Example:

```text
reader gets key K at generation g7, revision r41
writer deletes K:                         r42
writer recreates K:                       r43
CAS(K, expected = g7:r41) -> conflict, even if the value is unchanged
```

Replace the ambiguous combination of `expected: Option<MemoryVersion>` and
write mode with one condition:

| Condition | Meaning at the operation's atomic decision point |
|---|---|
| `Any` | No existence/version precondition |
| `Missing` | The addressed key is absent now; not proof that it was always absent |
| `Exists` | The addressed key exists now |
| `Match(version)` | The key exists with that exact scoped version |

Source insert/update/upsert map to these conditions, not separate backend
algorithms. Successful writes create new revisions even for equal values.
Deleting an absent key under a satisfied condition returns a confirmed no-change
result, not a fabricated deletion version. A failed condition returns structured
conflict evidence and does not consume a committed revision.

For SQLite, use `BEGIN IMMEDIATE` for read-condition-modify transactions. Resolve
retained operation evidence first; for a new operation, check the condition,
allocate revision, mutate data and write its receipt in one transaction.
A connection/process mutex is not cross-process correctness.
Distinguish writer-lock contention from a proven version conflict; never retry a
failed CAS as an unconditional write. In-memory clients perform the equivalent
decision and mutation atomically under their own state ownership.

## 3. Commit Evidence And Recovery

Writes expose the following semantic outcomes; names describe contracts rather
than imposing a second generic execution state machine:

| Outcome | Required evidence |
|---|---|
| `Committed(receipt)` | Backend confirms the mutation committed; receipt identifies the operation, resulting revision and actual durability contract |
| `NotCommitted(reason)` | Confirmed pre-dispatch rejection, condition conflict, no-change operation, or confirmed abort/rollback |
| `Unknown(operation_ref)` | Commit cannot be determined from available evidence; preserve a reference for later reconciliation |

Memory/session response types must carry these distinctions structurally.
Commit evidence describes the requested domain mutation. Persisting a conflict
receipt alone does not turn a rejected data write into `Committed`.
`HostError` text, `Result::is_ok()` and a generic cancellation flag cannot encode
commit certainty. Read operations have no write-commit receipt. A confirmed
conflict is a completed conditional operation without a mutation, not an
unknown backend failure.

The run outcome, write evidence and local cleanup outcome are independent:

```text
run = Cancelled
write = Committed(receipt)
cleanup = Settled
```

That combination is valid. Keep the receipt in trace/checkpoint evidence even
when the engine does not resume the ordinary source continuation. A local
timeout does not change an already confirmed commit into a rollback. Conversely,
receiving a response is not sufficient proof of commit if the response reports
uncertainty. Extend service-specific `OperationResponse` projection to preserve
this evidence rather than classifying all `Ok` responses identically.

SQLite durable adapters explicitly configure and verify `journal_mode=WAL`
and `synchronous=FULL` for the durable profile, using the backend/platform's
documented synchronization guarantees. Do not infer durability from WAL alone.
Unsupported settings fail initialization rather than silently downgrade.
In-memory receipts identify volatile completion. Other backends must declare
their own acknowledgement/durability contract, not emulate SQLite guarantees.

### 3.1 Idempotency And Reconciliation

When retry-safe writes are requested, use an operation/idempotency key scoped to
the backend namespace and logical operation. `HostRequestId` alone is not a
cross-restart identity. Reuse existing trace/operation correlation; do not create
a business transaction schema inside Host.

Persist the operation key, canonical request fingerprint and bounded receipt in
the same transaction as the write. A retry with the same key and request can
return the original receipt without another mutation; the same key with a
different request must fail. Fingerprints include target, operation, condition
and typed payload identity without leaking raw sensitive values into diagnostics.
Reconciliation must use an authoritative backend read under current authority.

Receipt lookup distinguishes confirmed completion from unresolved, expired or
unavailable evidence. Absence of a receipt is not by itself proof of non-commit:
it might have expired, or the original request might still be running. Do not
automatically retry an unknown write without an actual idempotency or
reconciliation guarantee. Applications choose the policy; Host enforces it.

Receipt storage has configured size and retention bounds. Retention of a receipt
is distinct from retention of its data. Do not promise permanent exactly-once
execution, and do not silently evict evidence while advertising a longer retry
window.

## 4. Managed Database Execution

Use a backend execution unit with a fixed number of connection-owning workers
and bounded admission. Its narrow responsibility is database work, not language
scheduling. The shared `ExecutionScope` remains the owner of each admitted job.

```text
validated request + effective limits
  -> bounded admission: job count and retained bytes
  -> registered job in a bounded backend queue
  -> exclusive connection owner executes domain statements
  -> transaction evidence and result recorded
  -> result delivery and cleanup observation
```

Acquire capacity before retaining additional encoded payload copies. Bound
waiting admissions as well as queued jobs; an unlimited semaphore wait list
would still retain unlimited requests. Saturation returns a structured overload
result or uses an explicitly bounded wait. Make queue, connection, busy-wait,
operation and cleanup limits configurable; operation deadlines constrain the
effective waits, while cleanup has its own protected deadline.

Reuse managed blocking ownership and registrations under `execution/`; do not
create a second cancellation token or fire-and-forget thread per request. A
worker owns its SQLite connection rather than many blocking workers polling one
`Mutex<Connection>`. Domain adapters supply their statements and result mapping;
the shared SQLite layer must not match on `MemoryOperation` or `SessionOperation`.

All public client entry points must use the same controlled path. An embedding
convenience entry may establish a managed operation context, but cannot bypass
limits by running synchronous SQL inside an async body. Reject new work during
shutdown. Receiver/Future drop must not detach a transaction or erase its eventual
outcome. Retain bounded completion evidence, not abandoned full result payloads.

### 4.1 Cancellation At The Transaction Boundary

| Point | Required behavior |
|---|---|
| Before admission/dispatch | No backend mutation; return confirmed non-commit |
| Waiting for connection or writer lock | Observe cancellation/deadline with bounded waits; do not label contention as version conflict |
| Executing statements | Use request-scoped progress/interrupt controls and loop budget checks; retain connection ownership until statements settle |
| Before starting commit | Honor an observed stop and perform protected abort/rollback |
| Commit in progress | Observe the actual outcome; a concurrent stop request does not prove rollback |
| Commit confirmed | Preserve receipt even if cancellation wins result delivery |
| Cleanup | Settle statements/transaction, remove cancellation hooks, and then release the connection; report pending/failed cleanup honestly |

A late interrupt must not affect the next connection user. Associate cancellation
with the active job identity, unregister/drain its callback and ensure no old
interrupt can run after handoff. Prevent interruption from racing connection
close. Progress callbacks cannot bound every lock or OS I/O wait, so those need
separate backend limits and documented remaining limitations.

Rollback is protected from the already-triggered business cancellation signal.
Explicitly observe rollback/transaction state rather than relying solely on
`Drop`. An autocommit flag alone cannot distinguish prior commit from rollback;
combine it with operation phase and backend evidence. If cleanup cannot be
confirmed, quarantine the connection and retain pending ownership instead of
returning it to the worker pool. A non-cooperative OS operation cannot be killed
by dropping its Rust Future; do not claim a hard termination bound without an
appropriate isolated execution backend.

## 5. Resource Limits And Stored Values

Effective bounds are the intersection of deployment ceilings, service settings
and request budgets. Omitted request limits use bounded defaults, never infinity.
Invalid limits fail validation instead of being silently changed to another
operation. Use checked arithmetic when accumulating sizes and counts.

| Resource | Enforcement point |
|---|---|
| Request/key/entry bytes | Type-directed engine conversion and host validation, before cloning or encoding large values |
| Encoded bytes | Bounded writer during serialization; no full temporary JSON tree |
| Decode depth/nodes/string/container sizes | Decoder checks while constructing values, including failed-decode cleanup |
| Persisted row/BLOB bytes | SQLite row/value limits and bounded column access before allocating an owned string/blob |
| Page bytes and entry count | Incremental row consumption before appending each decoded entry |
| Scanned rows/bytes and work time | Query execution even when most candidates do not match |
| Queue/result memory | Admission and delivery accounting, including simultaneous pending responses |
| Receipt/summary/cursor bytes | Construction, persistence and decoding, including provenance and metadata |

Consolidate duplicated tagged encoders/decoders from Memory and Session into
`value/`. Preserve numeric, bytes, collection, variant and message payload data
losslessly. Reuse the stored format through streaming serializer/deserializer
interfaces with explicit budgets; a new binary format is not required to bound
allocation. Use schema fingerprints and engine type-directed conversion where
the protocol requires them. Corrupt, incompatible or oversized values fail
explicitly, never become null, empty lists or default records.

Reads of pre-existing oversized data are covered, not just writes made by the
new adapter. Limit SQLite row/BLOB sizes; inspect actual stored lengths before
copying/decoding values, within a consistent read operation. Do not filter out
oversized rows and pretend they were missing. A selected entry too large for an
empty page is an explicit error, not a non-advancing empty cursor page.

Conflict results return version/existence evidence by default, not the current
stored payload. Returning a current value is an explicit read-authorized and
bounded operation. Error, debug and trace output must not expose stored content,
secrets or raw tokens merely to explain a conflict. Resource bounds do not
replace operator disk quotas or OS process memory isolation.

## 6. Pagination And Query Algorithms

Replace numeric OFFSET cursors with validated, versioned keyset cursors. Tokens
bind namespace, Store/session identity, ordering, query/context fingerprint and
continuation state. They do not grant authority. Reject malformed, foreign or
incompatible cursors; never restart from zero silently.

For ordinary Memory scan, use the stable canonical key ordering and a Store
revision fence. Read the fence and page in one read transaction. A subsequent
page whose Store revision changed returns explicit cursor invalidation, rather
than presenting mixed pages as a snapshot. This avoids retaining long-lived
transactions and unbounded historical versions. Do not describe these cursors
as resumable snapshots that survive arbitrary concurrent writes.

Push supported predicates/order/limits into bounded backend queries. Fetch rows
incrementally and budget unmatched candidates too; a result limit alone cannot
bound query work. If exact evaluation exceeds its work budget, return an explicit
incomplete/error outcome, never silently return an allegedly complete prefix.
Operations without continuation support cannot return a success-shaped truncated
answer.

Exact vector search uses a bounded top-k heap and deterministic canonical-key
tie-breaking, not collect-all followed by sort. Also bound embedding dimensions,
candidate decoding and total work. Exactness is promised only after the search
actually completes. Backends without a required query/vector/transaction feature
reject it explicitly, without emulating successful empty results.

## 7. Session Semantics

Reuse Session's own message and history model on top of shared execution, codec,
limit and transaction mechanisms. Do not turn Session into a second generic
key-value store or application lease manager.

- Resolve/create must report creation from the atomic database result, not a
  precheck raced against a later upsert. Re-resolving an identity must not
  silently overwrite another caller's configuration; configuration changes are
  explicit conditional operations.
- Append atomically allocates a persistent ordinal, enforces session-scoped
  message/dedup uniqueness and writes the receipt. Session ordinals do not reset
  when retained messages are deleted.
- The same dedup key with the same normalized logical append can reuse its
  receipt. Reusing it with a different payload or semantic metadata is a conflict,
  not success returning someone else's message.
- History paging fixes an upper ordinal and context/retention cutoff at the
  first page, then fetches bounded pages by ordinal. New appends above the upper
  bound do not extend that enumeration. Destructive retention, context publication or
  configuration changes invalidate affected cursors through explicit generation
  checks rather than silently reshaping history.
- Apply history selection in the database or bounded iterator, not by decoding
  all messages and then truncating. Context windows, summaries and provenance
  count toward the same result bounds.
- Context publication is a conditional data write of caller-produced content,
  not a request for the storage backend to run a summarizer or tokenizer.

Session receipts and dedup evidence have declared retention scopes. Deleting a
message does not silently preserve an unlimited duplicate copy in a receipt,
nor may it silently retain an exactly-once claim after deleting dedup evidence.

### 7.1 History Fence And Context Publication

History access returns bounded typed messages and backend-issued evidence for
the selected session generation, history revision, upper ordinal, selection
range/cutoff and expected context version. Bind pagination to that selection;
do not reconstruct evidence from caller-supplied counts or timestamps.

Use the four source APIs specified under `std.agent.session`:

| API | Architecture contract |
|---|---|
| `history_page` | Bounded typed messages, cursor, backend-issued `SessionHistoryFence` and optional published context |
| `prepare_context` | Validate/bind the session, fence and caller-produced `SessionContextContent` to a runtime-issued `StorageOperationRef`; no content generation or publication |
| `publish_context` | Validate that the supplied content/fence match the prepared operation, then conditionally publish and return a structured outcome |
| `reconcile_context` | Reauthorize query-only lookup using the same operation reference; never generate content or resubmit a publication |

Preparation may return the operation reference for later
`publish_context(session, fence, content, operation)`. It must bind the complete
canonical request; changing content, provenance, session or fence under the same
reference is an error, not a new publication. Do not collapse preparation into
dispatch or introduce a second generic `reconcile` entry point for Session.
Persist the prepared reference and necessary payload/fence explicitly when crash
recovery is needed. The publication algorithm is:

1. Validate authority and bounded content/provenance; fingerprint the session,
   fence and complete canonical content, not a summarization policy name.
2. In one transaction, look up matching operation evidence first. Return its
   recorded outcome on replay; reject reuse for a different request.
3. For a new operation, check session generation, selected history revision and
   expected context version together. An intervening append, deletion or context
   replacement conflicts; a stable paging upper bound alone is not publication
   authority. Do not merge stale content or silently recompute it.
4. Atomically publish the content with its provenance and new version, and record
   the receipt. Do not implicitly delete the original messages.

Strict conflict detection deliberately leaves re-reading, re-summarizing and
retention decisions to the caller. Receipt replay must still return an earlier
commit after the history advances; it must not re-run step 3 against that commit.
Unknown outcomes use query-only reconciliation. Fences and operation references
are bounded, checkpointable evidence, not authorization grants.

`SessionContextContent` preserves source-history and producer provenance. A
successful publication proves neither semantic accuracy nor a higher trust
level. Preserve trust/provenance through codecs and context selection; a stored
summary is not automatically trusted system-prompt content. Identity/fence
validation does not validate the truth of the producer's claims.

`SessionContextReceipt` exposes operation identity, session generation, published
context version and backend-confirmed durability. An in-memory receipt must not
claim persistent durability. Generation failure/cancellation leaves published
context unchanged; after publication dispatch, cancellation cannot prove rollback.

### 7.2 Context Processing Belongs To Applications

Etas provides typed history, context publication, ordinary checked model/tool
calls, usage accounting, budgets, cancellation and trace. EDK/application flows
compose these primitives:

```text
bounded history + revision fence
  -> application token counting / selection
  -> application summary flow or Agent call
  -> application output validation
  -> conditional context publication
```

The application selects its model/provider, tokenizer, prompts, quality rules,
retry policy and retention policy. Model calls run through the ordinary checked
Host boundary, outside database transactions and without holding a database
worker across inference. Provider usage remains distinguishable from an
application's estimated token count; no character-count substitute is reported
as exact model tokenization.

Remove storage-owned `SessionCompactor::count_tokens` / `summarize` orchestration
and implicit `CompactionPolicy::SummarizeWhen` execution from the target Etas
contract. Reuse their bounded history, validation and conditional-write mechanics
where applicable; do not retain a second hidden model-call path. A
`SummaryPlusRecent` selects already-published context and recent messages. With
no published summary, select recent turns only and expose the absent summary
explicitly, not as fabricated empty content. This is the SPEC-defined view, not
a claim that full history was returned. Selection neither deletes source
messages nor generates a summary. `ContextTokens(...)` remains a budget bound;
exhaustion follows the limit failure rules and never triggers hidden inference.

A production summarizer or tokenizer configuration is not a completion gate for
Etas core. Test providers may verify application composition, but must not be
advertised as a production Etas summarizer. The updated SPEC already removes
`SessionConfig.compaction`; retain its `id`, `context` and `retention` fields.
Remove obsolete declarations and dispatch now, with explicit migration errors
for old source/configuration rather than silently accepting ignored options.

Keep execution of configured retention, archival, deletion and storage
compaction under runtime/storage ownership. These bounded maintenance operations
do not invoke a model. They preserve required provenance/evidence and make any
loss of replay payloads explicit. Do not remove them merely because an old file
or helper contains the word `compact`; split mixed maintenance/summary logic by
responsibility. Publishing context alone never authorizes history deletion.

## 8. Standard Library And Engine Integration

### 8.1 Public Storage Workflow

Keep the existing `std.memory` namespace and atomic `get_entry` API. Session
operations remain in `std.agent.session`; do not add a forwarding `std.session`
module. The following are accepted contract signatures, to be recorded with
their concrete source declarations and error rows in the SPEC:

```text
get_entry<K,V>(store, key) -> Option<MemoryEntry<K,V>>
prepare_put<K,V>(store, key, value, condition) -> MemoryWriteIntent<K,V>
prepare_delete<K,V>(store, key, condition) -> MemoryWriteIntent<K,V>
commit<K,V>(intent) -> WriteOutcome<MemoryWriteReceipt<K>, MemoryWriteRejection>
reconcile<K,V>(store, operation_ref) -> ReconcileResult<MemoryWriteReceipt<K>, MemoryWriteRejection>
```

`MemoryEntry` carries the key, value and version from one read. Reuse existing
bounded `page`/cursor support, rather than introducing a competing
`get_versioned`/scan model. Write conditions retain `Any`, `Missing`, `Exists`
and `Match(version)` semantics from section 2.

An immutable `MemoryWriteIntent` binds the Store/key, mutation kind, typed
payload when present, condition and opaque operation reference. Expose a typed
read-only accessor for that reference so callers can persist it independently.
Preparation validates and allocates identity but does not mutate storage. This
uses runtime identity allocation and is not a deterministic pure builtin or an
application-chosen random string. Allocation failure is explicit.

Allocate identity before dispatch. Callers that need recovery across a crash
must durably save the intent/reference or take an explicit checkpoint before
commit. Merely returning an in-memory intent does not guarantee crash recovery.
Restore preserves the same identity; it does not allocate a new operation.
Intents contain no connections, worker handles, cancellation tokens or authority
grants. Restore, commit and receipt lookup all validate the current binding and
authority. A serialized operation reference alone grants no access.

### 8.2 Outcome And Reconciliation Types

Use closed tagged outcomes, not nullable fields or generic I/O errors:

```text
WriteOutcome<R,N> =
    Committed(R)
  | NotCommitted { operation, reason: N }
  | Unknown { operation }

ReconcileResult<R,N> =
    Found(ConfirmedOutcome<R,N>)
  | Unresolved
  | Expired

ConfirmedOutcome<R,N> = Committed(R) | NotCommitted { operation, reason: N }
```

Receipts identify the operation, scoped target, confirmed mutation/no-change
result and applicable version evidence. Deletion does not fabricate a live
entry version. Rejections distinguish condition conflicts, missing/existing
conditions and other proven non-commit outcomes without exposing unauthorized
stored values. `Unknown` requires inspection/reconciliation, not automatic retry.
Lookup is query-only: it cannot resubmit the mutation. `Unresolved` and `Expired`
are not evidence that nothing committed.

Commit validates identity and complete canonical request before replay. Matching
retained evidence wins over re-evaluating a now-stale CAS condition. A different
request under the same reference is rejected. Mutation and its receipt commit
atomically; confirmed rejection evidence is also bounded and retained according
to the declared receipt policy. Expiry cannot authorize treating an old operation
as new. No unbounded exactly-once guarantee is made across evidence loss, restore
or retention expiry.

### 8.3 Effects, Engines And Convenience APIs

Commit retains the scoped `Memory.write` requested action; reconciliation retains
the scoped read action. Versioned reads and paging use the same read authority.
Preparation must not claim a storage write occurred. Session publication and
lookup similarly use their declared write/read actions, not an implicit model
action. Std wrappers may handle substrate actions internally while preserving
them in requested-action summaries.

Pre-dispatch validation, permission and lookup failures use the declared typed
Error contract. Once dispatch can have mutated data, report certainty through
`WriteOutcome`, including `Unknown`; do not turn it into a retryable I/O error.
`?` only captures checked Error effects, not an outcome variant. Cancellation
remains run control while receipts still reach the operation ledger.

Retain ordinary `get`/`put`/`delete` conveniences over this same mechanism. They
may discard a successful receipt deliberately, but a typed uncertain-write error
must carry the operation reference if their established signature returns no
outcome. They cannot discard uncertainty, treat a conflict as success or use a
second mutation implementation. `keys` must follow bounded pages to completion
or return an explicit limit/error, never silently return the first page. A
multi-key `clear` requires a stable selection and conditional deletes; do not
page with invalidated cursors or delete newly replaced values unconditionally.

`MemoryVersion` is backend-issued evidence, not an integer callers increment.
An explicit token serialization/parser, if retained for transport, validates
syntax and bounds and does not invent a current Store binding. Receiving a
serialized token never bypasses normal Store/key and authority validation.

The interpreter and future runtime must preserve the approved result shapes
through their type-directed codec and route ordinary typed failures through
checked Error semantics. They must not guess versions or turn unknown writes
into retryable generic I/O failures. External run cancellation remains execution
control, not a catchable memory error; commit evidence still reaches its ledger.
Default handlers and `?` do not erase requested actions, write receipts or
resource-version observations.

No backend-specific SQL or receipt schema belongs in `etas_std` or the engines.
Test the real chain from source API and checked actions through engine dispatch,
Host admission, persistent adapter and typed response. Rust-only protocol tests
are insufficient to show the public contract works.

## 9. File Responsibilities And Migration

Target layout, retaining existing public service names:

```text
crates/etas_host/src/
  storage/
    mod.rs
    version/
      mod.rs
      token.rs                  # scope, generation, revision and token validation
      condition.rs              # shared condition semantics, not SQL
    transaction/
      mod.rs
      outcome.rs                # commit certainty and receipt contracts
      receipt.rs                # operation identity and reconciliation contract
    limits/
      mod.rs
      config.rs                 # validated ceilings and effective limits
      accounting.rs             # bounded admission, bytes and work accounting
    sqlite/
      mod.rs
      config.rs                 # verified connection/durability/size settings
      worker.rs                 # bounded connection-owning backend execution
      transaction.rs            # commit/rollback evidence and cleanup
      cancellation.rs           # active-job progress/interrupt registration
      receipt.rs                # atomic receipt persistence and lookup
  value/
    codec.rs                    # existing engine-facing codec contract
    tagged/
      mod.rs
      encode.rs                 # shared bounded lossless encoder
      decode.rs                 # budgeted parser, no intermediate full JSON tree
  memory/
    protocol.rs                 # StoreRef, operations and domain results
    client.rs                   # one managed dispatch contract
    in_memory.rs                # atomic conditions and equivalent supported bounds
    sqlite/
      mod.rs                    # client facade; no monolithic operation body
      schema.rs                 # Store metadata, entries and format migration
      read.rs                   # bounded get/version access
      write.rs                  # domain condition/mutation statements
      scan.rs                   # revision-fenced keyset traversal
      query.rs                  # bounded predicate/vector execution
  session/
    protocol.rs                 # message/session domain contract
    client.rs
    in_memory.rs
    context/
      mod.rs
      fence.rs                  # session/history/context version evidence
      publication.rs            # caller-produced content and write contract
    sqlite/
      mod.rs
      schema.rs                 # message, ordinal and history-generation schema
      append.rs                 # atomic resolve/append/dedup decisions
      history.rs                # bounded context and history traversal
      context.rs                # conditional context publication and receipts
      retention.rs              # explicit bounded deletions; no policy selection
```

Do not duplicate cancellation/lifecycle code under storage; backend controls
consume `execution` contexts. Shared SQLite helpers must not import memory/session
protocols. Keep operation-specific SQL in its service domain. Re-export existing
public version/cursor/client types as needed without retaining duplicate logic.

Replace the following old paths, not just their names:

1. Row-local `map_or(1, version + 1)` allocation and decimal-only version parsing.
2. Separate nullable expected-version and write-mode decision trees.
3. Direct synchronous SQL in an async client entry, and blocking Mutex polling
   as the database admission design.
4. Error remapping that overwrites commit evidence with the current cancel flag.
5. Memory-local duplicate tagged codecs and full-JSON-tree persistence decoding.
6. OFFSET pagination, full-history load-before-limit and collect-all vector sort.
7. Dedup success without checking whether the logical requests match.
8. Public API/result conversion that prevents users from observing versions or
   hides a conditional/unknown outcome.
9. Host compaction that calls a summarizer/tokenizer or selects a production
   model. Keep data-publication mechanics, move orchestration to EDK/application
   source, and remove superseded providers rather than keeping dormant dispatch.
10. CLI/runtime configuration requiring an automatic summarization provider.
    Remove obsolete configuration with an explicit migration diagnostic, not
    an ignored field or a fallback to concatenation/test providers.

Implement the now-specified Session API quartet, support types and removal of
implicit summarization across StdRegistry, generated stubs, type/effect facts,
package ABI and typed checkpoint codecs together. This work does not await a
production provider or further approval to delete the old summary callbacks.
Separately synchronize the general Memory intent declarations in section 8,
including full signatures, outcome fields and Error rows; do not treat the
Session update as approval of unrelated source APIs.
Unknown legacy intent/context formats must be rejected, not reconstructed with
new operation identities or unverified history fences.

Introduce a versioned database migration. Preserve valid stored values, assign
durable generations/revisions atomically and reject incompatible legacy tokens
and cursors explicitly. Multi-process startup must serialize migration before
serving operations. Failed migration must not silently recreate an empty store.
Tests use generated tokens rather than assuming every fresh key has version 1.

## 10. Acceptance Matrix

Use disposable databases and real adapters. The same semantic suite must cover
InMemory and SQLite where supported, with explicit durability differences.

| Area | Required evidence |
|---|---|
| CAS | Real concurrent connections/processes racing on one key; one winner, precise conflicts, no lost updates or unconditional retry |
| Version identity | Delete/recreate ABA, cross-key/Store tokens, missing/existing races, counter exhaustion, reopen and coordinated restore generation |
| Initialization | Concurrent schema migration/open, failure rollback, verified durability settings, no empty-store fallback |
| Limits | Oversized requests and pre-existing rows, malformed/deep values, excessive nodes/metadata, bounded peak allocation before rejection |
| Paging | Page byte/count bounds, invalid/foreign cursors, mutation invalidation, fixed Session upper bound and no silent data skipping |
| Queries | Unmatched candidate work budgets, exact top-k completion and explicit failure for unsupported/incomplete evaluation |
| Concurrency | Saturation, bounded queued bytes/waiters, worker failure, deadlock deadlines and unrelated-request isolation |
| Cancellation | Before queue/dispatch, lock wait, SQL execution, before/during/after commit, receiver drop, rollback failure and pending cleanup |
| Connection reuse | Late interrupt cannot cancel the next job or access a closed connection |
| Commit certainty | Real transaction with injected lost acknowledgement; independent read/receipt establishes truth, no blind retry of unknown writes |
| Intent lifecycle | Preparation causes no write; immutable payload/condition; checkpoint and restart retain identity; current authority required; same reference with a changed payload rejected |
| Session | Same-key same/different append, parallel writers, retention/publication races and receipt expiry semantics |
| Context publication | Concurrent append/context replacement conflicts; matching receipt replay succeeds after history changes; publication never invokes a model/tokenizer or deletes raw history |
| Session source API | All four APIs are callable; prepare binds the complete request without publishing; changed payload/fence rejected; receipt includes generation/version/durability; unknown outcomes use `reconcile_context` |
| Context selection | `SummaryPlusRecent` with/without a summary; visible absence, bounded recent-only view, no hidden inference or deletion; exhausted `ContextTokens` follows the normal limit error |
| Trust and maintenance | Publication preserves provenance and does not upgrade trust; configured retention/storage compaction still executes with explicit replay-data loss reporting |
| Application composition | Source flow reads bounded history, produces context via ordinary checked calls and publishes conditionally; conflict/unknown/cancellation remain observable |
| Security | No stored payloads in errors/traces; conflict payload requires read authority; malformed data never becomes a default value |
| Source chain | Approved std API -> checked effects -> interpreter -> SQLite -> typed version/receipt/page, including conflict and unknown outcomes |

Use synchronization barriers and fault-injection points, not sleeps as proof of
ordering. All subprocess tests have deadlines and assert exit/status/results;
observing an arbitrary failure is not sufficient. Fakes supplement but do not
replace real multi-process transactions or persisted malformed-value tests.
Process-crash tests demonstrate their particular failure model, not physical
power-loss durability on every filesystem. Record actual platform/backend
guarantees and keep unsupported ones explicit.

## 11. Practice References

- [SQLite transactions](https://www.sqlite.org/lang_transaction.html): write
  admission, `BEGIN IMMEDIATE`, and distinct contention/transaction outcomes.
- [SQLite interrupt](https://www.sqlite.org/c3ref/interrupt.html): cancellation
  races, connection lifetime, and interruption of concurrent statements.
- [SQLite synchronization](https://www.sqlite.org/pragma.html#pragma_synchronous):
  WAL and durability are separate configuration obligations.
- [SQLite limits](https://www.sqlite.org/limits.html): runtime row/BLOB/SQL bounds
  complement, rather than replace, application codec and result budgets.
