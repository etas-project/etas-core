# Storage binary profile, version 1

Implementation: `crates/etas_std/library/std/memory/binary.es`. Encoding,
decoding, size admission and canonical ordering are Etas algorithms. The
profile is used by the source `codec`, `key_codec` and `query` factories in
`std/memory/encoding.es`. Private runtime mechanisms capture typed values,
restore them against checked layouts and supply stable schema identities;
they do not encode bytes. Store protocol validation, derivation access
authorization, protected provenance and strict codec callback evidence remain
acceptance work. Portable default callable payloads, source execution and normal
SDK publication have been verified. The complete installed example executes
with the dependency-selected standard-library image, including all seven logical
collection adapters and an omitted query codec argument.

## Portable mutation outcomes

The source Store convenience mutations prepare an intent and submit it once.
The owning MemoryWriteIntent implementation translates confirmed conflicts and
rejections into `StorageError`, retaining the operation reference. An unknown
outcome raises `outcome_unknown` with `Some(operation)` so a caller can reconcile;
it does not retry. An unconditional delete treats an absent key as a successful
no-op. Clear captures entry versions before conditionally deleting them, and
propagates a failed mutation after any earlier committed deletions.

StorageError has `code`, `message` and `operation: Option<StorageOperationRef>`.
Host storage and session outcomes use the same record ABI. Portable receipt
durability is `Durable` or `Volatile`; the SQLite WAL mode remains backend
metadata. The compiler checks the source-owned error and operation-reference
layouts before admitting their runtime primitives.

Store prepares key/value bytes with the codecs above, and decodes reads, pages
and receipt keys in its Etas implementation. A codec view retains the original
description and combines its type schemas with codec formats/versions into a
persistent binding fingerprint. Host backends validate this fingerprint before
accessing existing data. SQLite format 4 adds that binding to store metadata;
older stores retain their previous binding and require an explicit migration
before a differently encoded view can use them.

Memory.read transports bytes and leaves Error<StorageError>. Memory.write now
submits the prepared operation and returns its outcome; the previous unit write
entry point is removed. Query payloads retain explicit language/version identity
through the host boundary. Backends without a registered adapter report
unsupported instead of interpreting the payload or generating an embedding.

Consolidated SDK source checking and volatile-backend execution have passed.
`examples/spec_storage_outcomes.es` exercises the source convenience policy with
explicit handlers; `examples/spec_storage_store.es` exercises binary CRUD,
receipts, reconciliation and codec views against a real backend. Selection
content limits currently report measure_unavailable when no resolved tokenizer
context is available; measurement-context integration remains acceptance work.

The installed Store example preserves a maximum u128 key, a u128 value and f32
NaN payload bits through commit/read/reconcile. The same codec view reads the
same versions and keys, and deferred scan, deletion and existence checks execute
against the backend. This validates binary CRUD integration; strict custom-codec
behavior, derivation permissions and protected provenance remain separate gates.

Returned codec callables publish their checked suspension result in portable
callable-value metadata. Source export, binary metadata decoding and re-export
retain it; an absent result remains unknown. The installed
`examples/spec_storage_codec.es` example calls the default encoder inside an
updating receiver method, where non-suspending execution must be proved, then
checks decoding and receiver writeback. This execution property is one part of
the full strict codec contract above.

Storage capture/restoration includes the checked source view/build adapters in
its suspension dependency graph. Hash implementations and those private
adapters must have deterministic, action-free, non-suspending bodies; imports
must retain the corresponding checked callable-body evidence. Generic Hash calls use
this validated member contract rather than treating any empty Effect row as
proof. `examples/spec_storage_hash.es` exercises a custom Hash implementation
through Map encoding and reconstruction inside an updating receiver method.

With the same installed Store example and a development CLI build, sharing the
checked runtime type index and simplifying source hexadecimal formatting reduced
evaluation from 203.2 to 82.0 seconds, and total time from 237.1 to 113.8 seconds.
Both runs completed all nine backend requests and retained the example's binary
round-trip assertions. SDK loading remained about 0.42 seconds. User-program
checking and interpreter continuation/frame costs remain performance work.

## Logical collection adapters

`std/collections/storage.es` owns the private `__storage_view` and
`__storage_build` methods inside each collection's `impl`. Their checked
method headers are published as internal compiler contracts; ordinary source
lookup retains their private visibility. They have Etas bodies rather than
native default implementations. The typed runtime walker invokes these
methods through normal checked calls. Capture retains the logical map/set
tags so canonical key ordering is performed by the Etas encoder.

The compiler records the storage runtime's implicit calls in the checked semantic
dependency graph: capture depends on the checked view implementations, and
restoration on the checked build implementations. These edges keep private
method bodies in dependency-selected SDK objects even though the source runtime
declarations contain no calls. Ordinary collection use does not add storage
callbacks to its dependency closure.

| Collection | Logical view | Reconstruction |
| --- | --- | --- |
| Map | Array of live key/value pairs in iteration order | Insert pairs through the owning Map implementation |
| Set | Array of members in iteration order | Insert members through the owning Set implementation |
| Slice | Array of elements within the view | Create a view covering the reconstructed array |
| Stack | Array in stack iteration order | Rebuild the list without reversing the observable order |
| Queue | Array in dequeue order | Rebuild the queue with the same sequence |
| Deque | Array from front to back | Rebuild and balance its private lists |
| BoundedDeque | Record of values, capacity and overflow policy | Check capacity/content agreement and rebuild the deque; return None if invalid |

Hash seeds, bucket capacity, removed slots, backing-array offsets and the
distribution between front/back lists are implementation details and are not
part of those logical views. BoundedDeque's capacity and overflow policy are
observable and are retained.

## Checked layouts and schema identity

`CheckedProgramData.describe_storage_layout` describes a concrete checked type
as a finite graph. `etas_types::storage_layout` materializes closed enum
payloads and logical adapter views, retaining each adapter's checked method
identity and owner bindings for subsequent execution preparation. An opaque
type without a representation requires an explicit codec. Protected trust
wrappers require a protected provenance implementation; this query does not
invent one.

Named types are referenced by canonical declaration identity and closed type
argument identities. Their representation schemas are recorded separately.
Anonymous shapes are hashed structurally, ignoring interner numbering and
graph sharing. Record fields and named enum fields are ordered by name; enum
variants are ordered by name. Native payload positions remain available for
reconstruction and do not participate in the schema hash. Function bodies and
collection implementation state are not schema inputs.

The query returns the extended type store containing any newly materialized
logical view types. It is descriptive compiler data, rather than permission
to invoke private constructors or export protected provenance. Runtime
capture/restoration uses the graph, and native invocation preparation
materializes its closed types and selected adapter applications when a storage
mechanism is reached. The query alone remains insufficient derivation evidence
for private constructors or protected provenance.

## Typed execution boundary

`StorageValue.capture`, `restore` and `schema` are private source declarations
for minimal runtime value operations. Their checked signatures and complete
StorageValue layout are validated together. The interpreter uses an owned
continuation to traverse the concrete layout and call collection adapters;
execution budgets and cancellation remain active across these calls. Layout
caches belong to one invocation and retain the checked program with each
program/type key.

The source factories use a fixed version-1 profile (1 MiB, 16384 nodes, depth
48), without observing a clock or mutable storage context. Encoding and decoding
are function-valued record fields, invoked as `(codec.encode)(value)` and
`(codec.decode)(bytes)`. Query construction validates language/version and
encodes its payload without executing a backend query.

The public example `examples/spec_storage_codec.es` exercises wide integers,
float bit patterns, all seven logical collection adapters, recursive enums,
canonical map keys and a default query codec. Public/Untrusted wrappers retain
their labels and source representation. Trusted/Sanitized derivation still
requires protected provenance and is rejected until that implementation is
available; the decoder cannot manufacture evidence.

## Framing

An encoded value is `45 54 53 01`, a mode byte (`00` for a value, `01` for a
key), a schema string and one node. Trailing bytes are invalid. Each string
or byte string is prefixed with its byte length as an unsigned 64-bit little
endian integer; strings contain strictly valid UTF-8. Sequence and field
counts use the same integer encoding. A schema is a stable structural type
identity supplied by checked derivation, never a process-local type index.

## Nodes

All tags below are hexadecimal. Fixed-width numeric payloads use little
endian raw bits, including two's complement signed integers and IEEE floating
point encodings. NaN payloads and signed zero are preserved.

| Tag | Payload |
| --- | --- |
| `00` | Unit, no payload |
| `01` | Boolean byte, `00` or `01` |
| `10`–`14` | i8, i16, i32, i64, i128: 1, 2, 4, 8, 16 bytes |
| `15` | isize: width byte (4 or 8), followed by that many bytes |
| `18`–`1c` | u8, u16, u32, u64, u128: 1, 2, 4, 8, 16 bytes |
| `1d` | usize: width byte (4 or 8), followed by that many bytes |
| `20`, `21` | f32, f64: 4 or 8 bytes |
| `22` | Unicode scalar: 4 bytes; surrogates are excluded |
| `30`, `31` | String or bytes |
| `40`–`47` | Tuple, array, list, slice, stack, queue, deque, set: count and nodes |
| `48` | Map: entry count and alternating key/value nodes |
| `50` | Record: field count and alternating field-name strings/value nodes |
| `51` | Nominal value: type-identity string and representation node |
| `52` | Enum: type-identity string, variant-name string, field count and nodes |
| `53` | Trust-labelled data: label byte, protected-provenance bytes, inner node |

Trust labels are `00` Public, `01` Untrusted, `02` Trusted, `03` Sanitized.
Parsing a `53` node creates private inert data. Only the checked runtime
restoration boundary may verify its protected provenance and construct a
corresponding language value. Parsing does not grant trust or authority.
The format does not serialize secrets, live resources or executable values.

Record fields are always ordered by the lexicographic order of their UTF-8
names, independently of construction-expression order. Duplicate names are
invalid. Nominal and enum identities are preserved alongside the root schema.

Value mode preserves logical sequence, map and set iteration order. Key
mode recursively sorts map entries by their canonical encoded key bytes and
set members by their canonical encoded element bytes. Ordered collections
retain their logical order in both modes. Duplicate map keys and set elements
are invalid in either mode; duplicates are identified using canonical key
bytes, including for nested maps/sets. Raw floats are excluded from keys.
Key encodings include all tags, field names, nominal identities and lengths;
concatenated payloads cannot be confused by missing delimiters.

## Admission and decoding

The encoder measures byte size, node count and nesting depth before building
the output or sorting encoded keys. The decoder checks its input byte budget
before parsing, bounds lengths by remaining input and child counts by the
remaining node budget before allocation, and checks depth on entry to a node.
Malformed data produces `Result.Err(StorageError)` without requesting Error
actions. Execution-budget and cancellation enforcement remain runtime work.

The decoder re-encodes the parsed representation and compares the bytes,
checking scalar validity, field ordering, uniqueness and key canonicalization
through the same source implementation. Typed restoration must additionally
validate the expected schema, nominal layout, collection invariants and
protected provenance. This private parser does not substitute for that step.

## Positive format vectors

Spaces separate bytes. The short schema strings here exercise framing;
installed derived codecs must supply their checked stable schema identities.

| Schema / value | Encoding |
| --- | --- |
| `u`, unit | `45 54 53 01 00 01 00 00 00 00 00 00 00 75 00` |
| `i8`, signed -1 | `45 54 53 01 00 02 00 00 00 00 00 00 00 69 38 10 ff` |
| `f32`, bits `7fc00001` | `45 54 53 01 00 03 00 00 00 00 00 00 00 66 33 32 20 01 00 c0 7f` |
| `u128`, maximum value | `45 54 53 01 00 04 00 00 00 00 00 00 00 75 31 32 38 1c ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff` |
