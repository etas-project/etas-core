# Phase 1 Core Design

## 1. Purpose

Phase 1 implements the Etas language frontend and AST/HIR interpreter. In this
phase, `etas-core` provides shared infrastructure, shared language contracts,
and reusable pure builtin kernels. It must stay free of frontend, interpreter,
runtime, and CLI ownership so dependent repositories can share contracts without
creating cycles.

## 2. Crates

```text
etas-core/
  crates/
    etas_core/
    etas_utils/
    etas_cache/      # generic artifact cache and dependency primitives
    etas_std/        # declaration-only standard language support surface
    etas_builtin/    # reusable pure builtin implementations
    etas_host/       # shared host protocols and reusable host adapters
```

Phase 1 should keep this repository small. The frontend, interpreter, and CLI
all depend on it, so every type placed here becomes part of the shared contract.
Prefer moving domain-specific concepts out of `etas-core` unless at least two
repositories need the same stable representation or pure implementation.

### 2.1 `etas_core`

Responsibilities:

- `SourceId`, `SourceFile`, and source text metadata;
- `TextSize`, `TextRange`, `Span`, `LineIndex`, and line/column conversion;
- shared `Diagnostic`, labels, notes, suggestions, and diagnostic results;
- typed ids, arenas, interner, and small result/error helpers.

Non-goals:

- no lexer/parser/AST;
- no HIR, type, or effect semantics;
- no interpreter values or execution state;
- no CLI formatting policy beyond reusable diagnostic data structures.

### 2.2 `etas_utils`

Responsibilities:

```text
fixpoint/
  lattice.rs
  transfer.rs
  worklist.rs
  solver.rs
  iteration.rs

graph/
  traversal.rs
  topo.rs
  scc.rs
  dominator.rs

pipeline/
  pass.rs
  pipeline.rs
  manager.rs
  analysis.rs
  artifact.rs
  preservation.rs
  instrumentation.rs
  schedule.rs
  config.rs
  result.rs
  timing.rs

pattern/
  responsibility_chain.rs
```

These utilities are generic. They may be used by frontend checking and
interpreter-local analysis, but they must not define Etas-specific type
domains, HIR facts, pass semantics, or evaluation semantics.

### 2.3 `etas_cache`

`etas_cache` owns generic artifact cache and dependency-tracking primitives for
incremental compiler sessions.

Responsibilities:

- artifact keys, cache namespaces, and artifact metadata;
- project revisions and fingerprints;
- dependency graph primitives between artifact keys;
- invalidation sets and invalidation selectors;
- memory artifact store for hot session artifacts;
- disk artifact store using SQLite metadata plus content-addressed binary blob
  payloads.

Non-goals:

- no frontend artifact semantics such as `HirModule`, `TypeFacts`, or
  `EffectFacts`;
- no parser, HIR, type, or effect semantics;
- no IDE indexes or LSP protocol data;
- no interpreter/runtime checkpoint storage.

`etas_frontend` defines frontend-specific artifact kinds and invalidation
meaning. `etas_cache` only provides reusable storage and dependency primitives.

See `docs/architect/etas-cache-design.md`.

### 2.4 `etas_std`

Phase 1 should introduce `etas_std` as a complete declaration registry for the
language surface described by the PL design documents. It describes what the
standard library exposes; it does not execute standard-library behavior.

Implementation may land in phases, but the data model must not encode a
reduced language surface. Primitive types, support types, effect tags, and
intrinsic descriptors should be represented with their complete intended shape
from the start so the frontend, interpreter, IDE, and future runtime share one
stable vocabulary.

- primitive type declarations;
- standard function signatures;
- prelude names;
- intrinsic descriptors and ids;
- documentation/completion metadata.

The implementation of pure builtins belongs in `etas_builtin`, not in
`etas_std` and not in the interpreter.

`etas_std` may materialize virtual source and HIR stubs from the registry for
LSP, dumps, diagnostics, documentation, and uniform symbol views. These stubs
are derived read-only views. The registry remains the canonical source for
standard signatures, effects, capabilities, intrinsic ids, and documentation
metadata; generated `.es` text must not become the authoritative standard
library input for Phase 1 compilation.

Recommended layout:

```text
crates/etas_std/
  src/
    lib.rs

    registry/
      mod.rs
      registry.rs
      builder.rs
      module.rs
      prelude.rs
      lookup.rs

    decl/
      mod.rs
      ty.rs
      function.rs
      effect.rs
      intrinsic.rs

    modules/
      mod.rs
      core.rs
      collections.rs
      option_result.rs
      text.rs
      bytes.rs
      json.rs
      math.rs
      agent.rs
      runtime.rs
      security.rs
      host.rs

    metadata/
      mod.rs
      docs.rs
      completion.rs
```

Core data model:

```rust
pub struct StdRegistry {
    modules: Vec<StdModule>,
    prelude: Prelude,
    by_path: HashMap<StdPath, StdItemId>,
}

pub struct StdModule {
    pub name: &'static str,
    pub items: Vec<StdItem>,
}

pub enum StdItem {
    Type(StdTypeDecl),
    Function(StdFunctionDecl),
    Effect(StdEffectDecl),
    Intrinsic(StdIntrinsicDecl),
}

pub struct StdFunctionDecl {
    pub path: StdPath,
    pub generics: Vec<StdGenericParam>,
    pub params: Vec<StdParam>,
    pub return_type: StdTypeRef,
    pub effects: StdEffectSet,
    pub intrinsic: Option<StdIntrinsicId>,
}

pub struct StdEffectDecl {
    pub path: StdPath,
    pub generics: Vec<StdGenericParam>,
    pub extends: Option<StdEffectRef>,
    pub actions: Vec<StdEffectActionDecl>,
}

pub struct StdEffectActionDecl {
    pub name: &'static str,
    pub generics: Vec<StdGenericParam>,
    pub params: Vec<StdParam>,
    pub return_type: StdTypeRef,
}
```

`etas_std` must not depend on frontend type definitions. It should define a
lightweight declaration type language that `etas-frontend` maps into its own
type checker representation:

```rust
pub enum StdPrimitiveType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    F32,
    F64,
    Char,
    String,
    Bytes,
    Unit,
    Never,
}

impl StdPrimitiveType {
    pub fn source_name(self) -> &'static str;
}

pub enum StdTypeRef {
    Primitive(StdPrimitiveType),
    Named(StdPath),
    GenericParam(&'static str),
    Applied {
        base: StdPath,
        args: Vec<StdTypeRef>,
    },
    Function {
        input: Vec<StdTypeRef>,
        output: Box<StdTypeRef>,
        effects: StdEffectSet,
    },
}
```

Primitive enum variants are Rust implementation names only. Source names,
diagnostics, dumps, and documentation must render PL names exactly: `bool`,
`i32`, `u32`, `f64`, `string`, `bytes`, `unit`, `never`, and the other concrete
numeric widths. `StdTypeRef` must not expose abstract `Int` or `Float` types.
Integer and floating-point behavior is expressed through concrete widths.

Registry construction should use a builder so module declarations stay
readable and centralized:

```rust
pub fn build_std_registry() -> StdRegistry {
    StdRegistryBuilder::new()
        .module(modules::core::module())
        .module(modules::collections::module())
        .module(modules::option_result::module())
        .module(modules::text::module())
        .prelude(modules::core::prelude())
        .build()
}
```

Example module declaration:

```rust
pub fn module() -> StdModule {
    StdModule::new("text")
        .ty("string")
        .function("len")
            .param("s", StdTypeRef::Primitive(StdPrimitiveType::String))
            .returns(StdTypeRef::Primitive(StdPrimitiveType::Usize))
            .pure_intrinsic(StdIntrinsicId::StringLen)
            .finish()
        .finish()
}
```

Usage by other Phase 1 repositories:

```text
etas_std
  -> declares standard names, types, function signatures, effects,
     intrinsic descriptors, and intrinsic ids

etas-frontend
  -> uses etas_std for prelude lookup, name resolution, type checking,
     and effect checking

etas-interpreter
  -> uses etas_std intrinsic descriptors and ids to call supported
     pure intrinsic kernels in etas_builtin

etas
  -> may use etas_std metadata for help text later, but does not own
     standard-library semantics
```

Interpreter dispatch should avoid stringly typed builtin execution:

```rust
match intrinsic_id {
    StdIntrinsicId::StringLen => builtins::string_len(args),
    StdIntrinsicId::ListLen => builtins::list_len(args),
    StdIntrinsicId::OptionIsSome => builtins::option_is_some(args),
    StdIntrinsicId::ResultIsOk => builtins::result_is_ok(args),
}
```

Standard declaration surface:

```text
primitive types:
  bool
  i8 i16 i32 i64 i128 isize
  u8 u16 u32 u64 u128 usize
  f32 f64
  char
  string
  bytes
  unit
  never

core generic types:
  Array[T]
  List[T]
  Map[K, V]
  Set[T]
  Range[I]
  Slice[T]
  Deque[T]
  Queue[T]
  Stack[T]
  PriorityQueue[T, P]
  OrderedMap[K, V]
  OrderedSet[T]
  Option[T]
  Result[T, E]

compiler-known support constraints:
  Index

trust/security support:
  Trusted[T]
  Untrusted[T]
  Secret[T]
  Public[T]
  Sanitized[T]

agent/runtime support:
  Prompt
  PromptPart
  PromptEncode[T]
  Schema[T]
  ResponseDecode[T]
  Message[T]
  SessionConfig
  Conversation
  SandboxProfile
  ApprovalRequest
  ApprovalDecision
  Limit

core effects:
  Inference
  Network
  FileIO
  Command
  Memory
  Secret
  Time
  Human
  Error[E]

standard effect actions:
  Console extends FileIO:
    stdin_read_line
    stdin_read_all
    stdout_write
    stderr_write
  Memory:
    read[R]
    write[R]
    migrate[R]
    compact[R]
  Approval extends Human:
    request

pure intrinsics:
  primitive arithmetic and comparison for concrete numeric widths
  concrete primitive parse/string conversion helpers
  string.len
  string.lines
  string.split
  string.trim/lowercase/uppercase
  string-list.join
  list.len
  map.len
  map.contains_key
  set.len
  option.is_some
  option.is_none
  result.is_ok
  result.is_err
  assert
  abort

prelude:
  primitive type names
  Array
  List
  Map
  Set
  Range
  Slice
  Option
  Result
  Some
  None
  Ok
  Err
  unit
  assert
  abort
  Trusted
  Untrusted
  Secret
  Public
  Sanitized
  Prompt
  Message
  Sandbox
  DefaultCommandSandbox
  Iterations
  Tokens
  ContextTokens
  Cost
  WallTime
  Attempts
  approve
  core effect names
```

Phase 1 `etas_std` may declare IO, agent, tool, memory, provider, time,
checkpoint, and host-authority support contracts when they are part of the PL
design. It must not execute them. The Phase 1 frontend/interpreter should emit
explicit unsupported-runtime diagnostics when a checked program requires
behavior that belongs behind the future AIR/runtime authority boundary.

### 2.4 `etas_builtin`

`etas_builtin` owns reusable implementations for pure deterministic intrinsic
kernels described by `etas_std`. It is shared by the HIR interpreter in Phase 1
and the future AIR runtime, so the same operation has one implementation and
one error policy.

Responsibilities:

- dispatch pure intrinsic ids described by `etas_std`;
- implement primitive arithmetic and comparison for concrete numeric widths;
- implement deterministic text, bytes, collection, option, result, assertion,
  abort, and pure JSON helpers;
- enforce builtin-local errors such as overflow, divide-by-zero, invalid
  slicing, invalid UTF-8, and invalid argument shapes;
- expose structured builtin errors that interpreter/runtime layers can map to
  their own diagnostics or runtime failures.

Non-goals:

- no HIR or AST evaluation;
- no AIR scheduling;
- no interpreter frames, local slots, captures, or control signals;
- no runtime authority checks;
- no model, tool, memory, filesystem, network, time, randomness, approval, or
  checkpoint execution;
- no CLI formatting policy.

Recommended layout:

```text
crates/etas_builtin/
  src/
    lib.rs

    dispatch/
      mod.rs
      registry.rs
      call.rs

    value/
      mod.rs
      value.rs
      adapter.rs
      type_tag.rs

    numeric/
      mod.rs
      int.rs
      uint.rs
      float.rs
      compare.rs
      convert.rs

    text/
      mod.rs
      string.rs
      char.rs

    bytes/
      mod.rs
      ops.rs

    collections/
      mod.rs
      list.rs
      map.rs
      set.rs

    option_result/
      mod.rs
      option.rs
      result.rs

    json/
      mod.rs
      parse.rs
      stringify.rs

    control/
      mod.rs
      assert.rs
      abort.rs

    error.rs
```

The crate should not depend on `etas-interpreter` value types or future
runtime value types. It should use a small builtin-local value boundary and
adapters:

```rust
pub enum BuiltinValue {
    Unit,
    Bool(bool),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    F32(f32),
    F64(f64),
    Char(char),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<BuiltinValue>),
    List(Vec<BuiltinValue>),
    Map(Vec<(BuiltinValue, BuiltinValue)>),
    Set(Vec<BuiltinValue>),
    Range {
        start: Box<BuiltinValue>,
        end: Box<BuiltinValue>,
        bounds: RangeBounds,
    },
    Slice(Vec<BuiltinValue>),
    OptionSome(Box<BuiltinValue>),
    OptionNone,
    ResultOk(Box<BuiltinValue>),
    ResultErr(Box<BuiltinValue>),
}

pub enum RangeBounds {
    ClosedOpen,
    OpenClosed,
}

pub trait BuiltinValueAdapter {
    type Value;
    type Error;

    fn into_builtin(value: Self::Value) -> Result<BuiltinValue, Self::Error>;
    fn from_builtin(value: BuiltinValue) -> Result<Self::Value, Self::Error>;
}

pub fn call_pure_intrinsic(
    intrinsic: StdIntrinsicId,
    args: &[BuiltinValue],
) -> Result<BuiltinValue, BuiltinError>;
```

The enum above is a dispatch boundary, not the whole language value model.
Interpreter-specific values such as frames, closures, local slots, and control
signals stay in `etas-interpreter`. Runtime-specific values such as host
handles, trace references, checkpoint references, and action-grant tokens stay
in `etas-runtime`.

`BuiltinValue` is the shared pure intrinsic value protocol. It must preserve the
source-level distinction between `Array[T]` and `List[T]`; adapters may share
storage internally, but they must not silently coerce between the two. `Range`
stores bounds and inclusivity explicitly. `Slice` represents the value visible to
pure builtins; an interpreter or runtime may hold a more efficient view
internally as long as the adapter exposes value semantics.

Adapter implementations may be connected intrinsic by intrinsic, but the crate
API and error taxonomy should be shaped for the complete primitive surface.

### 2.5 `etas_host`

`etas_host` owns shared host-facing protocol values and reusable host adapters
for external capabilities such as model calls and tool invocation. It exists so
the Phase 1 checked-HIR interpreter and future AIR runtime can use the same
external protocol surface without sharing their execution loops or internal
value models.

The important boundary is:

```text
shared:
  HostValue
  ModelRequest / ModelResponse
  ToolRequest / ToolResponse
  MemoryRequest / MemoryResponse
  ConsoleRequest / ConsoleResponse
  AuthorityContext
  TraceContext
  Budget
  provider/tool/memory/console protocol adapters such as OpenAI, MCP, HTTP tools,
  SQLite, Postgres, vector-store adapters, and fake/std process streams

not shared:
  InterpValue <-> HostValue codec
  AirValue <-> HostValue codec
  HIR effect/action -> HostRequest lowering
  AIR effect instruction -> HostRequest lowering
  HIR memory API call -> MemoryRequest lowering
  AIR memory instruction -> MemoryRequest lowering
  interpreter evaluator
  runtime scheduler, checkpointing, continuation, and authority enforcement
```

`etas_host` may define provider adapters, but those adapters operate only on
host protocol types. They must not depend on HIR, AIR, interpreter frames,
runtime scheduler state, CLI rendering, or frontend type/effect facts.

Recommended layout:

```text
crates/etas_host/
  src/
    lib.rs

    value/
      mod.rs
      host_value.rs
      schema.rs
      codec.rs

    request/
      mod.rs
      id.rs
      context.rs
      error.rs

    transport/
      mod.rs
      http.rs
      sse.rs
      retry.rs
      timeout.rs
      auth.rs

    model/
      mod.rs
      protocol.rs
      client.rs
      openai.rs
      anthropic.rs
      local.rs

    tool/
      mod.rs
      protocol.rs
      client.rs
      mcp.rs
      http.rs
      process.rs

    memory/
      mod.rs
      protocol.rs
      client.rs
      sqlite.rs
      postgres.rs
      vector.rs

    authority/
      mod.rs
      grant.rs
      action.rs
      approval.rs
      sandbox.rs
      policy.rs

    trace/
      mod.rs
      context.rs
      event.rs

    budget/
      mod.rs
      token.rs
      time.rs
      cost.rs
```

Host protocol values are engine-neutral:

```rust
pub enum HostValue {
    Unit,
    Bool(bool),
    Int(i128),
    UInt(u128),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<HostValue>),
    Map(Vec<(HostValue, HostValue)>),
    Record(Vec<(String, HostValue)>),
    Variant {
        name: String,
        fields: Vec<HostValue>,
    },
    Json(HostJsonValue),
}

pub trait HostValueCodec<V> {
    type Error;

    fn encode(value: &V) -> Result<HostValue, Self::Error>;
    fn decode(value: HostValue) -> Result<V, Self::Error>;
}
```

`HostValue` is intentionally protocol-shaped rather than language-shaped.
External model/tool/network protocols commonly expose JSON-like arrays and
objects, not Etas `Array[T]` versus `List[T]` semantics. The interpreter and
future runtime must use type-directed codecs when crossing the host boundary:

```text
InterpValue::Array[T]  <-> HostValue::List, guided by expected Array[T]
InterpValue::List[T]   <-> HostValue::List, guided by expected List[T]
AirValue::Array[T]     <-> HostValue::List, guided by expected Array[T]
```

`etas_host` must not guess whether a host list is an Etas `Array` or `List`.
That decision belongs to the engine-specific codec using frontend type facts,
schema descriptors, or explicit adapter configuration.

The trait is shared; implementations are not. The HIR interpreter implements it
for interpreter values. The AIR runtime implements it for runtime values.

Model protocol shape:

```rust
pub struct ModelRequest {
    pub id: HostRequestId,
    pub provider: Option<ModelProviderId>,
    pub model: ModelName,
    pub messages: Vec<ModelMessage>,
    pub tools: Vec<ToolSchema>,
    pub options: ModelOptions,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

pub struct ModelResponse {
    pub id: HostRequestId,
    pub message: ModelMessage,
    pub tool_calls: Vec<ModelToolCall>,
    pub usage: Option<ModelUsage>,
}

pub trait ModelClient {
    type Error;

    async fn complete(
        &self,
        request: ModelRequest,
    ) -> Result<ModelResponse, Self::Error>;
}
```

Provider adapters such as `model::openai::OpenAiClient` implement
`ModelClient` by translating `ModelRequest` and `ModelResponse` to and from the
provider API. They do not know whether the request came from HIR interpretation
or AIR execution.

Provider adapters must be real clients, not only protocol envelopes.
`OpenAiClient` must issue a real OpenAI-compatible HTTP request such as
`POST {base_url}/chat/completions`; `AnthropicClient` must issue a real
Anthropic-compatible messages request. Both clients must map provider JSON
responses and provider errors into `ModelResponse` or rendering-neutral
`HostError`.

The following is not sufficient:

```text
base_url constants only
encode_request returning an envelope around ModelRequest
decode_response returning a prebuilt ModelResponse
TcpStream port smoke tests without ModelClient::complete
```

Default tests should use fake/local transports for deterministic request and
response mapping. Live local model tests should be opt-in and should call the
concrete `ModelClient::complete` implementation against local omlx:

```text
OpenAI-compatible:    http://127.0.0.1:8848/v1
Anthropic-compatible: http://127.0.0.1:8848
small model:          Qwen3.5-0.8B-MLX-4bit
```

Tool protocol shape:

```rust
pub struct ToolRequest {
    pub id: HostRequestId,
    pub tool: ToolRef,
    pub args: HostValue,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

pub struct ToolResponse {
    pub id: HostRequestId,
    pub result: Result<HostValue, HostError>,
}

pub trait ToolClient {
    type Error;

    async fn invoke(&self, request: ToolRequest) -> Result<ToolResponse, Self::Error>;
}
```

Adapters such as MCP, HTTP, or process-backed tools can be implemented under
`etas_host::tool` as long as they remain protocol adapters. They may map
`ToolRequest` to an external protocol and `ToolResponse` back to `HostValue`,
but they must not decide language semantics or bypass authority context.

Tool adapters should likewise implement concrete `ToolClient` behavior where
the protocol is reusable. HTTP and MCP tool clients belong in `etas_host`.
Process-backed clients may also live here as protocol adapters, but actual
permission to execute a process remains caller-owned.

Typed persistent-memory protocol shape:

```rust
pub struct MemoryRequest {
    pub id: HostRequestId,
    pub store: StoreRef,
    pub operation: MemoryOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

pub struct MemoryResponse {
    pub id: HostRequestId,
    pub result: Result<MemoryResult, HostError>,
}

pub trait MemoryClient {
    type Error;

    async fn execute(&self, request: MemoryRequest) -> Result<MemoryResponse, Self::Error>;
}
```

`etas_host::memory` owns engine-neutral references such as
`MemoryRegionRef`, `StoreRef`, `MemoryVersion`, and `MemoryConflict`, plus
backend adapters where the protocol is reusable. SQLite is appropriate for
local tests and prototypes, Postgres for server deployments, and vector-store
adapters for retrieval memory. These adapters must not depend on HIR, AIR,
interpreter frames, runtime scheduler state, or Etas type/effect inference.

Usage by execution engines:

```text
etas-interpreter:
  InterpValue <-> HostValue
  checked HIR effect/action/API call -> ModelRequest / ToolRequest / MemoryRequest
  limited Phase 1 behavior may reject runtime-required requests explicitly

etas-runtime:
  AirValue <-> HostValue
  AIR effect instruction -> ModelRequest / ToolRequest / MemoryRequest
  runtime scheduler enforces authority, checkpointing, tracing, and budgeting
```

`etas_host` is therefore the shared protocol and adapter layer; execution
policy remains in the engine that uses it.

## 3. Dependency Direction

```text
etas_core
  no Etas crate dependencies

etas_utils
  -> etas_core

etas_std
  -> etas_core

etas_builtin
  -> etas_core
  -> etas_std

etas_host
  -> etas_core
  -> etas_std
```

Downstream Phase 1 repositories depend on `etas-core`:

```text
etas-frontend    -> etas-core
etas-interpreter -> etas-core
etas             -> etas-core
```

`etas-core` must never depend back on those repositories.

## 4. Public Contracts Needed By Phase 1

`etas-core` should expose stable enough contracts for:

- frontend diagnostics;
- interpreter diagnostics;
- CLI diagnostic rendering;
- source map references from HIR back to source spans;
- fixture and golden-test support.

The important invariant is that a diagnostic emitted by `etas-frontend` or
`etas-interpreter` can be rendered by `etas` without either component
depending on CLI code.

## 5. Recommended Public API Shape

Source and span:

```rust
pub struct SourceFile {
    pub id: SourceId,
    pub path: Option<PathBuf>,
    pub text: String,
    pub line_index: LineIndex,
}

pub struct Span {
    pub source: SourceId,
    pub range: TextRange,
}
```

Diagnostics:

```rust
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}
```

Typed ids and arenas:

```rust
pub trait Idx: Copy + Eq + Ord + std::hash::Hash {}

pub struct Arena<I, T> {
    // dense storage keyed by typed ids
}
```

The concrete Rust definitions can evolve, but this repository should provide
the shared source/diagnostic/id vocabulary used by every Phase 1 component.

## 6. Test Direction

`etas-core` tests should cover:

- span and text range arithmetic;
- byte offset to line/column conversion;
- diagnostic label ordering and rendering-neutral data shape;
- arena insertion and typed id stability;
- interner identity and lookup behavior;
- generic utility algorithms in `etas_utils`;
- host protocol value, schema, request, trace, authority context, and adapter
  request/response mapping behavior in `etas_host`.

Tests here should not import frontend syntax fixtures. If a test needs Etas
syntax, it belongs in `etas-frontend` or cross-repository integration tests.

## 7. Phase 1 Non-Goals

`etas-core` should not include:

- AIR execution contracts;
- runtime scheduler, checkpoint, continuation, or AIR instruction dispatch;
- engine-owned host lowering from HIR or AIR;
- interpreter/runtime value codec implementations;
- authority enforcement or approval workflow execution;
- LSP protocol types;
- interpreter frame/value models;
- frontend AST/HIR node definitions.
