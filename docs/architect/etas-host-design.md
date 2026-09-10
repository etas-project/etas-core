# Etas Host Design

Status: `Draft`

Owner: `Architect`

Last updated: `2026-09-10`

## 1. Purpose

`etas_host` is the shared crate for host-facing protocol values, reusable
host adapters, and engine-neutral execution lifecycle mechanisms.

It is shared by:

- the Phase 1 checked-HIR interpreter;
- the future AIR runtime;
- future IDE/testing tools that need to inspect host requests without executing
  them.

It does not execute Etas semantics. It defines the common boundary used when an
Etas execution engine needs to talk to external host services such as a model,
tool adapter, typed persistent-memory backend, human approval system, sandbox,
console, filesystem/network broker, or tracing sink.

The latest PL SPEC removes source-level `Capability`. `etas_host` may define
host grants and action request envelopes for runtime mediation, but those grants
are not importable language values and must not be confused with source-level
capabilities.

The core rule:

```text
Provider adapters and execution-scope lifecycle mechanisms are shared.
Engine value adapters, evaluators, and language schedulers remain engine-owned.
```

In concrete terms:

```text
shared:
  ModelRequest <-> OpenAI API <-> ModelResponse
  ToolRequest  <-> MCP/HTTP/process tool <-> ToolResponse
  MemoryRequest <-> SQLite/Postgres/vector store <-> MemoryResponse
  ConsoleRequest <-> stdin/stdout/stderr adapter <-> ConsoleResponse

not shared:
  HIR interpreter state <-> ModelRequest
  AIR runtime state     <-> ModelRequest
  HIR interpreter value <-> HostValue
  AIR runtime value     <-> HostValue
```

## 2. Crate Position

```text
etas-core/
  crates/
    etas_core/
    etas_std/
    etas_builtin/
    etas_host/
```

Dependency direction:

```text
etas_host -> etas_core
etas_host -> etas_std

etas-interpreter -> etas_host
etas-runtime     -> etas_host
```

`etas_host` must not depend on `etas-frontend`, `etas-interpreter`,
`etas-runtime`, `etas-optimizing`, `etas-ide`, or `etas`.

## 3. Ownership

`etas_host` owns:

- host boundary values;
- model request and response protocol;
- tool request and response protocol;
- typed persistent-memory request and response protocol;
- console/std-stream request and response protocol for `std.io`;
- command request and response protocol for `Command.run[S]`;
- low-level standard substrate protocols, owned by the corresponding host
  service domains: `network` for `std.net.tcp`, `stream` for `std.stream`,
  `tls` for `std.tls`, `filesystem` for `std.fs`, `secret` for `std.secret`,
  and `browser` for `std.browser.protocol`;
- session/conversation storage protocols used by agent execution, replay, and
  checkpoint metadata;
- external admission-adapter protocols used by runtime admission checks; the
  language trace-spec facts themselves still come from `etas_effects`;
- HTTP transport primitives used by host adapters;
- reusable provider clients such as OpenAI-compatible, Anthropic-compatible,
  and local model clients;
- reusable tool adapters such as MCP, HTTP, and process-backed tool protocol
  adapters;
- reusable memory backend adapters where the protocol is generic enough, such
  as SQLite, Postgres, and vector-store style adapters;
- bounded database execution, scoped versions/conditional writes and explicit
  commit evidence shared by Memory and Session, as specified in
  [Bounded Versioned Storage](etas-storage-design.md);
- opened-directory workspace bindings, handle-relative filesystem access and
  platform isolation contracts, plus explicitly staged snapshot/diff primitives,
  as specified in [Workspace Binding And Isolation](etas-workspace-design.md);
- sandbox policy values and reusable sandbox brokers for filesystem, network,
  and command execution;
- action grant and authority context values;
- approval request and decision values;
- console/stdin/stdout/stderr request values and reusable client interfaces;
- trace context and host trace events;
- budget values for tokens, time, and cost;
- execution scopes, cancellation propagation, operation ownership and shutdown
  observation, as specified in [Shared Execution Lifecycle](etas-execution-design.md);
- rendering-neutral host errors.

`etas_host` does not own:

- HIR evaluation;
- AIR execution;
- AST/HIR to host-request lowering;
- AIR instruction to host-request lowering;
- Etas memory schema derivation from `MemoryRegion[S]` or `Store[K, V]`;
- Etas memory effect inference such as `Memory.read[R]` or `Memory.write[R]`;
- interpreter or runtime value models;
- interpreter or runtime value codec implementations;
- scheduling of language computation and HIR/AIR tasks;
- checkpoint/resume implementation;
- continuation machinery;
- language-level trace-spec decisions, approval decisions, or grant derivation;
- provider configuration discovery policy;
- application workspace leases, takeover, persistent logical identity and
  recovery/retention decisions;
- application storage schema, retry/reconciliation policy, backup and disk quotas;
- CLI output formatting.

## 4. Internal Layout

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
      tagged/
        mod.rs
        encode.rs
        decode.rs

    context/
      mod.rs
      request/
        mod.rs
        id.rs
        context.rs
        error.rs
      authority/
        mod.rs
      budget/
        mod.rs
        execution.rs
      trace/
        mod.rs
        event.rs

    execution/
      mod.rs
      cancellation/
        mod.rs
        source.rs
        reason.rs
      scope/
        mod.rs
        identity.rs
        state.rs
        registry.rs
      operation/
        mod.rs
        context.rs
        registration.rs
        outcome.rs
      shutdown/
        mod.rs
        policy.rs
        report.rs
        wait.rs

    transport/
      mod.rs
      http.rs
      sse.rs
      retry.rs
      timeout.rs
      auth.rs
      network_policy.rs

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
      in_memory.rs
      sqlite/
        mod.rs
        schema.rs
        read.rs
        write.rs
        scan.rs
        query.rs
      postgres.rs
      vector.rs

    storage/
      mod.rs
      version/
        mod.rs
        token.rs
        condition.rs
      transaction/
        mod.rs
        outcome.rs
        receipt.rs
      limits/
        mod.rs
        config.rs
        accounting.rs
      sqlite/
        mod.rs
        config.rs
        worker.rs
        transaction.rs
        cancellation.rs
        receipt.rs

    console/
      mod.rs
      protocol.rs
      client.rs
      local_stdio.rs

    command/
      mod.rs
      protocol.rs
      client.rs
      local.rs

    policy/
      mod.rs
      protocol.rs
      client.rs
      local.rs
      http.rs

    session/
      mod.rs
      protocol.rs
      client.rs
      in_memory.rs
      sqlite/
        mod.rs
        schema.rs
        append.rs
        history.rs
        compact.rs
      retention.rs

    filesystem/
      mod.rs
      protocol.rs
      client.rs
      local.rs

    network/
      mod.rs
      protocol.rs
      client.rs
      tcp.rs

    stream/
      mod.rs
      protocol.rs
      client.rs

    tls/
      mod.rs
      protocol.rs
      client.rs

    secret/
      mod.rs
      protocol.rs
      client.rs

    browser/
      mod.rs
      protocol.rs
      client.rs

    sandbox/
      mod.rs
      policy.rs
      broker.rs
      workspace/
        mod.rs
        root.rs
        path.rs
        registry.rs
        tests.rs
      filesystem.rs
      network.rs
      command.rs
      snapshot.rs
      diff.rs
      platform/
        mod.rs
        requirements.rs
        activation.rs

    testing/
      mod.rs
      fake_transport.rs
      fake_sandbox.rs
      fixtures.rs
      assertions.rs
```

Layering:

- `value` defines shared wire-facing values, schemas and bounded lossless
  stored-value codecs. Memory and Session do not maintain duplicate codecs.
- `context` defines request ids, action-grant context, approval values, trace
  context, budget values, and rendering-neutral errors.
- `execution` owns the shared live scope/cancellation/operation/shutdown
  contract. It does not execute handlers or schedule language tasks; adapters
  keep their concrete stop/cleanup implementation in their service domain.
- `transport` defines reusable HTTP/SSE/auth/timeout/retry mechanics used by
  provider and tool clients, including network allowlist checks.
- `model` defines model protocols and model provider clients.
- `tool` defines tool protocols and reusable tool clients.
- `memory` defines typed persistent-memory protocols and reusable backend
  clients.
- `storage` defines shared version/condition contracts, resource limits,
  transaction evidence and managed SQLite execution. It does not import Memory
  or Session operation enums, interpret HIR/AIR, or define application state.
  Database statements remain in their service domain; cancellation ownership
  continues to come from `execution`.
- `console` defines console/std-stream protocols used by `std.io` declarations,
  including stdin reads and stdout/stderr writes.
- `command` defines the source-visible command execution host protocol for
  `Command.run[S]`. `sandbox::command` checks profiles and policies; it does
  not own the command request protocol.
- `policy` is a runtime-facing admission-adapter namespace, not a source-level
  language policy module. It can ask an external policy service or local
  adapter whether a concrete action is admissible, but it must not duplicate
  frontend effect inference or trace-spec fact materialization.
- `session` defines reusable session/conversation storage protocols for agent
  execution, replay, and checkpoint metadata. It is storage and protocol
  plumbing, not continuation or scheduler ownership. History paging and append
  deduplication follow [Bounded Versioned Storage](etas-storage-design.md#7-session-semantics).
- `filesystem` defines the source-visible host protocol for `std.fs` and local
  workspace-scoped filesystem implementations. Existing filesystem clients
  should be extended here; do not add parallel `fs` protocol types elsewhere.
- `network` defines the source-visible host protocol for `std.net.tcp`. It must
  not become a high-level HTTP client surface. Existing URL/HTTP request clients
  belong under `transport` when they are internal provider/tool plumbing, or in
  EDK/package code when they are source-visible high-level APIs.
- `stream` defines the source-visible byte-stream protocol for `std.stream`.
- `tls` defines the source-visible TLS session protocol for `std.tls`.
- `secret` defines the source-visible host secret protocol for `std.secret`.
- `browser` defines the source-visible browser protocol/session transport for
  `std.browser.protocol`.
- `sandbox` defines workspace boundaries and reusable safety brokers for
  filesystem, network, and command execution. `sandbox::workspace` owns the
  shared region-to-opened-root registry; `sandbox::platform` distinguishes
  required restrictions from backend-activated guarantees. Service domains
  retain their protocols and concrete operation ownership.
- `testing` defines fake transports, fake sandboxes, fixtures, and assertions
  used to test host behavior without touching the user's real system.

Top-level modules should stay coarse and should map to real host service
domains. Do not add a generic `substrate/` top-level directory on top of
existing domains; it creates duplicate `filesystem`/`fs` and `network`/`net`
paths. `filesystem`, `network`, `stream`, `tls`, `secret`, and `browser` own the
source-visible standard substrate protocols. `sandbox::filesystem`,
`sandbox::network`, and `sandbox::command` own safety checks and policy
mechanics used by those domains. Request, authority, trace, budget, and errors
belong under `context`. `console` is a separate top-level host domain because
stdin/stdout/stderr are process-console services, not workspace filesystem
services.

Protocol envelope types alone are not a complete implementation. A model
adapter is complete only when it implements `ModelClient` and can issue a real
request to a compatible endpoint. A tool adapter is complete only when it
implements `ToolClient` and can invoke the configured external tool protocol. A
memory adapter is complete only when it implements `MemoryClient` and can
execute the configured backend operation through the supplied authority,
trace, budget, schema, and version constraints.

A console adapter is complete only when it implements `ConsoleClient` and can
execute stdin/stdout/stderr operations through the supplied authority, trace, and
budget context. `LocalStdioClient` is one local implementation. Console clients
must be testable with fake input/output buffers so interpreter, runtime, and CLI
tests do not touch the user's real terminal unless explicitly wired by the
user-facing `etas` command.

### 4.1 Execution Lifecycle Integration

The authoritative state machine, structures, file responsibilities and tests
are in [Shared Execution Lifecycle](etas-execution-design.md). All host request
kinds receive its local `OperationContext`; this live context is not serialized
to external providers. Existing authority, trace and budget contracts remain
distinct. Request examples below show domain payloads, not an exhaustive live
execution envelope.

Extend existing service clients and supervisors to observe the same scope signal.
Remove superseded per-service invocation cancellation protocols; do not create a
second Host scheduler or duplicate cancellation under `context/`. Preserve
resource-specific ownership: cancelling a console wait does not stop the shared
stdin broker, and cancelling one operation does not implicitly close a shared
stream. Backend work must remain observable until actual local completion.

An adapter reports external completion evidence independently of local cleanup.
Cancellation is neither rollback nor proof that a remote model/tool stopped.
Database work must use managed blocking execution and a backend interruption/
completion strategy, not synchronous I/O inside an async body or a detached
`spawn_blocking` call. A cleanup timeout leaves the operation owned and pending.

## 5. Host Value Boundary

`HostValue` is the shared protocol value. It is not the interpreter value model
and not the AIR runtime value model.

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
```

Host schemas are used for model tool calls, external tool validation, and
diagnostics:

```rust
pub enum HostSchema {
    Unit,
    Bool,
    Int,
    UInt,
    Float,
    String,
    Bytes,
    List(Box<HostSchema>),
    Map {
        key: Box<HostSchema>,
        value: Box<HostSchema>,
    },
    Record(Vec<HostFieldSchema>),
    Variant(Vec<HostVariantSchema>),
    Json,
}
```

Codec shape:

```rust
pub trait HostValueCodec<V> {
    type Error;

    fn encode(value: &V) -> Result<HostValue, Self::Error>;
    fn decode(value: HostValue) -> Result<V, Self::Error>;
}
```

The trait is shared. Implementations are engine-owned:

```text
etas-interpreter implements HostValueCodec<InterpValue>
etas-runtime     implements HostValueCodec<AirValue>
```

This keeps host adapters reusable while allowing interpreter and runtime values
to stay different.

Storage-facing conversions and the stored-value codec additionally enforce the
[storage resource contract](etas-storage-design.md#5-resource-limits-and-stored-values).
Validate byte/depth/node budgets before cloning large values and during encoding
and decoding. A complete temporary JSON tree followed by a size check does not
meet this contract. The engine-facing sketch above is not permission to bypass
limits, type-directed decoding or sensitive-data handling.

`HostValue` is protocol-shaped, not source-language-shaped. It may use
`List(Vec<HostValue>)` for JSON arrays, model tool arrays, and HTTP payloads, but
it must not decide whether that value is an Etas `Array[T]` or `List[T]`.
Interpreter and AIR runtime codecs must use expected frontend types, schemas, or
adapter metadata to preserve source semantics:

```text
InterpValue::Array[T] <-> HostValue::List, guided by expected Array[T]
InterpValue::List[T]  <-> HostValue::List, guided by expected List[T]
AirValue collection   <-> HostValue::List, guided by AIR/schema type
```

## 6. Model Protocol

Model requests are engine-neutral.

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

pub struct ModelMessage {
    pub role: ModelRole,
    pub content: Vec<ModelContent>,
}

pub enum ModelRole {
    System,
    User,
    Assistant,
    Tool,
}

pub enum ModelContent {
    Text(String),
    Value(HostValue),
}

pub struct ModelResponse {
    pub id: HostRequestId,
    pub message: ModelMessage,
    pub tool_calls: Vec<ModelToolCall>,
    pub usage: Option<ModelUsage>,
}
```

Provider adapters implement a common client trait:

```rust
pub trait ModelClient {
    type Error;

    async fn complete(
        &self,
        request: ModelRequest,
    ) -> Result<ModelResponse, Self::Error>;
}
```

Example sharing:

```text
etas-interpreter:
  InterpValue -> HostValue -> ModelRequest
  ModelRequest -> OpenAiClient -> ModelResponse
  ModelResponse -> HostValue -> InterpValue

etas-runtime:
  AirValue -> HostValue -> ModelRequest
  ModelRequest -> OpenAiClient -> ModelResponse
  ModelResponse -> HostValue -> AirValue
```

The OpenAI adapter is shared because it only translates `ModelRequest` and
`ModelResponse` to and from the provider protocol. The value codec and engine
semantics remain separate.

### 6.1 Concrete Model Clients

`etas_host` must implement real model clients, not only request/response
envelopes.

Required clients:

```text
model::openai::OpenAiClient
  implements ModelClient
  sends HTTP POST to {base_url}/chat/completions
  maps ModelRequest into OpenAI-compatible JSON
  maps OpenAI-compatible JSON into ModelResponse

model::anthropic::AnthropicClient
  implements ModelClient
  sends HTTP POST to {base_url}/v1/messages or the configured compatible path
  maps ModelRequest into Anthropic-compatible JSON
  maps Anthropic-compatible JSON into ModelResponse
```

Required transport support:

```text
transport::HttpTransport
  request method, URL, headers, JSON body, timeout
  response status, headers, body bytes/string
  maps transport failures into HostError

transport::Auth
  bearer token
  x-api-key style token
  no-auth local mode

transport::Timeout
  connect timeout
  request timeout

transport::RetryPolicy
  disabled by default
  opt-in bounded retry for transient provider failures
```

The first implementation may use a standard Rust HTTP client such as `reqwest`
or another maintained crate. The choice is an implementation detail, but
`etas_host` must expose Etas-owned transport/client types rather than leaking
HTTP-client-specific types through its public API.

OpenAI-compatible request mapping must include at least:

```text
model
messages
temperature when present
max_tokens / max_completion_tokens when present
tools when present
```

OpenAI-compatible response mapping must extract at least:

```text
assistant message text/content
tool calls when present
usage when present
provider errors into HostError
```

Anthropic-compatible request mapping must include at least:

```text
model
system messages or equivalent system content
user/assistant messages
max_tokens when present
temperature when present
tools when present
```

Anthropic-compatible response mapping must extract at least:

```text
assistant content blocks
tool use blocks when present
usage when present
provider errors into HostError
```

The following does not count as a finished model adapter:

```text
base_url constants only
encode_request returning a wrapper around ModelRequest
decode_response returning a prebuilt ModelResponse
TcpStream port smoke test without chat/message completion
tests that never call ModelClient::complete
```

## 7. Tool Protocol

Tool requests are also engine-neutral.

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

    async fn invoke(
        &self,
        request: ToolRequest,
    ) -> Result<ToolResponse, Self::Error>;
}
```

Reusable adapters under `etas_host::tool` can support MCP, HTTP, or
process-backed tools as protocol adapters.

They may:

- translate `ToolRequest` into the external tool protocol;
- validate or encode `HostValue` arguments using `HostSchema`;
- translate external responses back into `ToolResponse`;
- preserve request ids, trace context, and rendering-neutral errors.

They must not:

- decide whether an action grant or policy allows execution;
- bypass approval requirements;
- mutate interpreter/runtime state directly;
- render CLI diagnostics;
- depend on HIR or AIR.

### 7.1 Concrete Tool Clients

`etas_host` should provide real reusable tool clients where the protocol is
generic enough to share:

```text
tool::http::HttpToolClient
  implements ToolClient
  sends ToolRequest to a configured HTTP endpoint
  encodes HostValue arguments as JSON
  decodes JSON result into HostValue

tool::mcp::McpToolClient
  implements ToolClient
  uses MCP request/response shapes
  keeps process/session ownership configurable outside language semantics

tool::process::ProcessToolClient
  implements ToolClient only if process execution is explicitly allowed by the
  caller-owned host controller
```

`etas_host` may provide the client implementation and protocol mapping, but it
must not decide whether a process, network endpoint, or MCP server is permitted.
The caller supplies authority context and the execution engine or host
controller enforces it.

## 8. Typed Persistent Memory Protocol

Typed persistent memory is also a host boundary. The Etas source language
models memory through ordinary types such as `MemoryRegion<S>` and
`Store<K, V>`, then binds immutable resource handles with top-level `let`.
`etas_host` does not derive those types and does not infer memory effects. It
only owns the engine-neutral protocol used when an execution engine talks to a
concrete backend.

The boundary follows the same pattern as model and tool support:

```text
etas-interpreter:
  checked HIR + type/effect facts
    -> MemoryRequest
    -> MemoryClient
    -> MemoryResponse
    -> InterpValue

etas-runtime:
  AIR memory instruction + runtime state
    -> MemoryRequest
    -> MemoryClient
    -> MemoryResponse
    -> AirValue
```

The authoritative protocol, algorithms and file responsibilities are in
[Bounded Versioned Storage](etas-storage-design.md). Retain the existing
`MemoryClient`/`SessionClient` boundaries and domain references, while replacing
the old nullable-version/mode and success/error-only write contracts.

| Contract | Required behavior |
|---|---|
| Request | Preserve backend/region/Store/schema identity, action authority, trace/request identity and effective byte/work/time limits |
| Version | Opaque Store generation plus Store-wide revision; no reuse after entry deletion/recreation and no per-process generation reset |
| Conditional mutation | One `Any / Missing / Exists / Match(version)` condition, checked atomically with the write/delete |
| Write evidence | Structurally distinguish confirmed commit with receipt, confirmed non-commit and unknown commit outcome; operation identity exists before dispatch |
| Reconciliation | Reauthorize scoped lookup; query retained evidence without replaying a mutation; missing/expired evidence is not non-commit |
| Conflict | Typed condition/version evidence; returning the current value requires explicit read authority and bounds |
| Read/page | Typed values with versions; bounded keyset continuation with declared revision/context consistency |
| Execution | One managed, bounded database path for every entry point; no synchronous-SQL async bypass |
| Encoding | Shared bounded lossless codec, including reads of pre-existing oversized/corrupt data |

`MemoryQuery` is a host query description, not an Etas expression tree. The
backend declares supported predicates, ordering, vector and transaction
features; an unsupported request fails explicitly. Exact vector queries use
bounded top-k selection and a work budget, not a full collection followed by
sorting. An incomplete search is not a successful complete answer.

SQLite is a persistent local adapter with actual cross-process transaction
semantics, not a single-client mock. Domain SQL lives under `memory/sqlite/`
and `session/sqlite/`; connection ownership, verified durability settings and
transaction cleanup live under shared `storage/sqlite/`. PostgreSQL and vector
adapters may implement the same protocols where supported, but must document
their actual token, consistency and durability guarantees. A backend-native
MVCC field alone is not proof of the required token scope or ABA protection.

The engine supplies checked action grants and type-directed conversions; Host
does not infer `Memory.read<R>` / `Memory.write<R>` or execute HIR/AIR. Session
uses the same storage mechanisms without losing its message, dedup and history
semantics. Host provides bounded typed history and conditional publication of
caller-produced context against a history/context revision fence. Publication
and its receipt are atomic; concurrent history changes conflict, and publication
does not implicitly delete messages.

The current Session SPEC names `history_page`, `prepare_context`,
`publish_context` and `reconcile_context`. Bind prepared content/fence to the
operation reference before dispatch, retain provenance/trust and report actual
durability in the receipt. Publication is not a semantic validation or trust
upgrade. `SummaryPlusRecent` selects only existing context; absent summary means
a recent-only view with visible absence, not an implicit model request.

EDK/applications choose schema, context selection, retention, summarization,
tokenization and recovery policy. Host does not call a production summarizer or
tokenizer from Session storage. Such calls use ordinary checked model/tool
services in application flows outside database transactions. No production
summarizer configuration is required to complete the Etas Host implementation.
Host/runtime still executes configured retention, archival, deletion and storage
compaction, preserving required evidence and reporting replay limitations. Only
semantic summary generation moves out; do not delete storage maintenance code.

Cancellation cannot erase a confirmed receipt or prove rollback. Unknown writes
must not become generic retryable errors, unconditional retries or empty-store
fallbacks. Preserve commit evidence through shared execution reporting and
engine checkpoint/trace records. The
[public API contract](etas-storage-design.md#8-standard-library-and-engine-integration)
retains `get_entry` and bounded pages and adds immutable `prepare_put` /
`prepare_delete` intents, `commit` and query-only `reconcile`. The engine
allocates/preserves operation identity and converts typed intent payloads; Host
owns canonical request validation and atomic evidence persistence. The full
general Memory intent declarations/errors still require SPEC synchronization.
Session publication and removal of `SessionConfig.compaction` are already in
the SPEC; delete the old model-driven callbacks without a compatibility branch.

### 8.1 Memory Authority

Typed persistent-memory access is authorized through checked action grants:

```rust
pub enum HostActionGrant {
    Allow(ActionPattern),
}

pub enum ActionPattern {
    Exact(ActionInstance),
    Pattern {
        effect: String,
        action: String,
        args: Vec<ActionArgPattern>,
    },
}
```

For memory this means grants such as:

```text
Memory.read<ProjectMemory>
Memory.read<ProjectMemory.Papers>
Memory.write<ProjectMemory.Drafts>
```

`etas_host` defines reusable request/grant values. The interpreter/runtime
decide how checked effect facts, deployment manifests, approval records, and
active policies produce the grants.

### 8.2 Memory Tests

Default memory tests should be deterministic and local:

- fake `MemoryClient` request/response roundtrips;
- request id, authority, trace, and budget preservation;
- `HostValue` key/value encoding;
- version scope, atomic conditions, deletion/recreation and concurrent conflicts;
- denied authority mapped to `HostError`;
- sqlite adapter tests using temporary database files only;
- vector adapter protocol tests with fake transport or in-memory fixtures.

The [storage acceptance matrix](etas-storage-design.md#10-acceptance-matrix)
is mandatory: real multi-process SQLite CAS, bounded allocation and paging,
commit/cancellation races, lost acknowledgements, Session dedup and the public
source-to-adapter-to-typed-result chain. Fake protocol tests alone do not
establish these guarantees.

No default test should connect to a user's production database or external
vector service. Live backend tests must be opt-in and require explicit
environment configuration.

## 9. Console And Standard Streams

`std.io` is a source-language standard module, but executing it is a host
boundary. `etas_host` owns reusable protocol values for process-console access
so the checked-HIR interpreter and future AIR runtime can share the same action,
trace, budget, and test vocabulary.

Recommended protocol:

```rust
pub struct ConsoleRequest {
    pub id: HostRequestId,
    pub operation: ConsoleOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

pub enum ConsoleOperation {
    ReadAllStdin,
    ReadLineStdin,
    WriteStdout { text: String, newline: bool },
    WriteStderr { text: String, newline: bool },
}

pub struct ConsoleResponse {
    pub id: HostRequestId,
    pub result: ConsoleResult,
}

pub enum ConsoleResult {
    Input(String),
    Written,
}

pub trait ConsoleClient {
    async fn execute(&self, request: ConsoleRequest)
        -> Result<ConsoleResponse, HostError>;
}
```

Action mapping:

```text
std.io.read_all   -> escaping [Error[IOError]], action Console.stdin_read_all
std.io.read_line  -> escaping [Error[IOError]], action Console.stdin_read_line
std.io.print      -> escaping [Error[IOError]], action Console.stdout_write
std.io.println    -> escaping [Error[IOError]], action Console.stdout_write
std.io.eprintln   -> escaping [Error[IOError]], action Console.stderr_write
```

`Console extends FileIO` in the effect lattice, so broad `FileIO` policy can
cover console operations. Host mediation remains narrower: console reads/writes
touch process standard streams, while filesystem access touches workspace or
host paths. The host controller may implement both using OS resources, but
action grants and readiness checks must remain separate so programs can be
granted stdout without granting arbitrary file access.

`etas_host` reports protocol and adapter failures as `HostError`. The
interpreter and future runtime map console `HostError` values into the
language-level `Error[IOError]` action using checked standard-library
descriptors. `etas_host` must not depend on HIR, AIR, or interpreter value
types to construct `Error[IOError]` directly.

Default console tests should use in-memory fake input/output buffers and assert:

- request id, trace, budget, and authority preservation;
- newline behavior for `print`, `println`, and `eprintln`;
- denied console action grant returns structured `HostError`;
- no test writes to the user's real terminal unless explicitly configured by the
  user-facing `etas` command.

## 9.5 Standard Substrate Host Services

The language SPEC accepts low-level standard substrate APIs that EDK default
handlers and user packages may build on:

```text
std.net.tcp
std.stream
std.tls
std.fs
std.secret
std.browser.protocol
```

These operations are standard substrate conceptually, but the code must not add
a generic `etas_host::substrate` directory. Each operation belongs to the
corresponding host service domain. The execution engine still decides whether a
checked action is allowed. `etas_host` receives an `AuthorityContext`, performs
the low-level operation through the configured broker/client, preserves trace
and budget metadata, and returns rendering-neutral host results.

The mapping is:

| Standard API | Host module | Host protocol | Authority action |
|---|---|---|---|
| `std.net.tcp.connect` | `etas_host::network` | `TcpConnectRequest` / `TcpConnectResponse` | `Net.tcp_connect[host, port]` |
| `std.stream.read/read_until_limit/write_all/flush/close` | `etas_host::stream` | `StreamRequest` / `StreamResponse` | `Stream.*[stream]` |
| `std.tls.connect` | `etas_host::tls` | `TlsConnectRequest` / `TlsConnectResponse` | `Tls.handshake[server_name]` |
| `std.fs.read_bytes/write_bytes/list/stat/atomic_replace` | `etas_host::filesystem` | `FilesystemRequest` / `FilesystemResponse` | `Fs.*[path]` |
| `std.secret.read` | `etas_host::secret` | `SecretRequest` / `SecretResponse` | `Secret.read[key]` |
| `std.crypto.hmac_sha256[K]` and other opaque-secret operations | `etas_host::secret` | `SecretUseRequest` / `SecretUseResponse` or `SecretOperation::Use*` | `Secret.use[K]` |
| `std.browser.protocol.*` | `etas_host::browser` | `BrowserProtocolRequest` / `BrowserProtocolResponse` | `Browser.attach/send/recv/screenshot/close[...]` |

Implementation must converge on one protocol per host service domain:

- `filesystem/` is the home for `std.fs`; duplicate `substrate/fs.rs`,
  `FsClient`, or `FsRequest` APIs should be removed or folded into
  `FilesystemClient` / `FilesystemRequest`.
- `network/` is the home for `std.net.tcp`; high-level HTTP request/response
  clients should not be exposed as the source-visible network substrate.
- `stream/`, `tls/`, `secret/`, `browser/`, `command/`, `policy/`, and
  `session/` are separate top-level domains because they are distinct host
  services, not sandbox policy modules.
- `sandbox/` supplies checks and brokers used by these services; it does not
  own their source-visible request protocols.

This layer is source-visible substrate, unlike `transport::HttpTransport`,
which is an internal reusable client facility for model, tool, and provider
adapters. For example, an OpenAI client may use `transport::HttpTransport`
internally without creating a source-level `Net.tcp_connect` action in the user
program. A user or EDK flow that calls `std.net.tcp.connect` does create the
checked `Net.tcp_connect` action.

Recommended request envelope shape is shared by convention, not by a mandatory
top-level `SubstrateRequest` type:

```rust
pub struct DomainRequest<T> {
    pub id: HostRequestId,
    pub operation: T,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

pub enum TcpOperation {
    Connect {
        host: String,
        port: u16,
        options: TcpOptions,
    },
}

pub enum StreamOperation {
    Read {
        stream: StreamRef,
        max_bytes: usize,
        timeout: Option<Timeout>,
    },
    ReadUntilLimit {
        stream: StreamRef,
        limit: ByteLimit,
        timeout: Option<Timeout>,
    },
    WriteAll {
        stream: StreamRef,
        body: Vec<u8>,
    },
    Flush {
        stream: StreamRef,
    },
    Close {
        stream: StreamRef,
    },
}

pub enum TlsOperation {
    Connect {
        stream: StreamRef,
        server_name: String,
        config: TlsConfig,
    },
}

pub enum FilesystemOperation {
    Read {
        path: WorkspacePathRef,
    },
    Write {
        path: WorkspacePathRef,
        contents: Vec<u8>,
        create_dirs: bool,
    },
    Delete {
        path: WorkspacePathRef,
    },
    ReadDir {
        path: WorkspacePathRef,
    },
    Stat {
        path: WorkspacePathRef,
    },
    AtomicReplace {
        path: WorkspacePathRef,
        contents: Vec<u8>,
    },
}

pub enum SecretOperation {
    Read {
        key: SecretKeyRef,
    },
    HmacSha256 {
        key: SecretKeyRef,
        body: Vec<u8>,
    },
}

pub enum BrowserProtocolOperation {
    Attach {
        profile: BrowserProfileRef,
    },
    Create {
        profile: BrowserProfileRef,
    },
    Send {
        session: BrowserSessionRef,
        message: BrowserProtocolMessage,
    },
    Recv {
        session: BrowserSessionRef,
        limit: BrowserEventLimit,
    },
    Screenshot {
        session: BrowserSessionRef,
        limit: BrowserPayloadLimit,
    },
    Close {
        session: BrowserSessionRef,
    },
}
```

Filesystem request payloads carry `WorkspacePathRef`, a region and relative
path, not a root handle supplied by the caller. Host registry binding produces
the internal `WorkspacePath` used by filesystem and command implementations.

`StreamRef`, `BrowserSessionRef`, and `SecretValue[K]` are opaque host handles.
They may be serializable as trace references, but they must not expose raw OS
file descriptors, raw sockets, raw browser process handles, or secret bytes in
diagnostics.

`StreamRef` must preserve provenance from the host action that created it. A
stream created by `std.net.tcp.connect` or `std.tls.connect` remains covered by
`Network`; a stream created by a future file-stream API remains covered by
`FileIO`. The stream service records `Stream.read/write[stream]` while retaining
origin metadata for policy and trace projection.

`StreamResponse` must distinguish ordinary EOF from data and typed errors.
`Read` returns a `StreamRead::Data(bytes)` or `StreamRead::Eof` equivalent; EOF
is not an error. Timeout, cancellation, closed streams, limit overflow, and host
failures are typed `StreamError` failures. `ReadUntilLimit` returns accumulated
bytes if EOF arrives first and fails on timeout, cancellation, limit overflow, or
host failure.

Filesystem substrate must resolve authorized region bindings and operate
relative to retained directory handles, enforcing path/link restrictions during
the actual operation. Canonicalization of a locator followed by ambient access
is not a confinement mechanism. Network and browser substrate must
pass through the sandbox network policy before opening sockets or connecting to
protocol endpoints. Secret substrate must redact by default in traces,
checkpoints, and diagnostics.

Pure standard helpers are not host protocols:

```text
std.http.codec.*
std.codec.text.*
std.crypto.sha256
std.crypto.constant_time_eq
```

Interpreter/runtime layers may dispatch those through `etas_builtin` or their
own deterministic value adapters. `etas_host` only participates when an
operation crosses a real host boundary such as secret retrieval, filesystem,
secret-backed crypto through `Secret.use[K]`, network, TLS session setup, or
browser protocol transport. `std.http.codec` operates on std-owned wire-level
`HttpWire*` types, not EDK's high-level `HttpRequest` / `HttpResponse` records.

Standard substrate host-service tests must include:

- deny-by-default authority tests for every substrate action;
- request id, trace, budget, and action-pattern preservation;
- timeout and cancellation behavior for TCP, stream, TLS, and browser protocol
  requests;
- bounded read behavior for streams;
- root/parent replacement, symlink escape, handle-relative atomic publication
  and explicitly staged rollback behavior for `std.fs`;
- secret redaction in debug output, trace payloads, and checkpoint-like
  snapshots;
- browser session/origin binding and denial of unapproved profile/session use;
- fake clients for deterministic unit tests and opt-in live tests only when a
  local or explicitly configured service is available.

## 10. Sandbox And Workspace

The authoritative identity, operation, platform, recovery and test contracts
are in [Workspace Binding And Isolation](etas-workspace-design.md). These are
shared Host mechanisms, not new source keywords or application workspace
management. The default filesystem posture is deny-by-default and explicitly
workspace-scoped.

The split is:

```text
etas_host owns:
  WorkspaceRoot
  WorkspaceRegionId / WorkspacePathRef / WorkspacePath
  WorkspaceRegionRegistry
  staged WorkspaceSnapshot / WorkspaceDiff mechanisms
  SandboxPolicy
  SandboxBroker
  filesystem/network/command sandbox adapters
  handle-relative path confinement and verified platform activation
  reusable test fakes and assertions

interpreter/runtime own:
  deriving action grants from checked program facts and deployment manifests
  deciding whether approval is required
  deciding whether a request may proceed
  enforcing the caller's staged-change commit/abort decisions
  mapping sandbox failures into language-level diagnostics or runtime failures

applications own:
  logical workspace identity and provisioning
  lease/fencing, takeover, recovery and retention policy
  associating approvals with conversation turns and workspace changes
```

`etas_host` therefore provides runtime admission mechanics for supplied
trace-spec/admission context and sandbox profile, but it does not decide
language authority. The caller chooses active trace specs, action grants,
approval records, and sandbox profiles; `etas_host` performs low-level safety
checks and returns rendering-neutral results.

Required binding rules:

- `WorkspaceRoot` retains a private opened directory binding; a canonical path
  is only its initial locator/display text, never its continuing authority.
- A checked static region, the opened OS object and an application's durable
  logical workspace identity are distinct. Host grants bind the actual opened
  root. `same-file`, path equality and copied identity files must not silently
  transfer those grants to a new binding.
- Root rename/replacement cannot retarget an existing binding. Reopen/resume
  creates a newly authorized binding, not a reconstructed OS handle.
- `WorkspaceRegionRegistry` is shared by filesystem and command domains.
  Duplicate registration fails without replacing the previous root or grants;
  missing regions do not fall back to the current directory.
- Resolve relative paths and perform operations through retained root/parent/
  leaf handles. Lexical normalization or a canonical-prefix check alone cannot
  prevent time-of-check/time-of-use races.
- `CommandSandbox` program admission and descriptor-bound child cwd do not
  establish process isolation. Required restrictions must be activated by a
  real backend before execution. Configuration labels are not enforcement
  evidence; missing guarantees fail explicitly without an unrestricted retry.
- Child descriptor inheritance is an explicit allowlist. Root and unrelated
  service handles must not leak to the executed program.

Default policy:

```text
filesystem read:  deny except explicit workspace root
filesystem write: deny except explicit workspace root
delete:           deny by default, or move-to-trash inside workspace
path traversal:   always reject
symlink escape:   always reject
network:          deny except explicit allowlist
command:          deny unless explicit command sandbox/action grant is configured
secret access:    deny unless explicit Secret.read/use action grant exists
approval:         required for destructive or authority-expanding requests
```

A directory handle is not a snapshot, transaction or lease. Snapshot/diff and
rollback guarantees apply only to explicitly staged changes. Ordinary directory
copies cannot promise snapshot consistency under concurrent writers. Atomic
replace publishes one entry; it does not imply multi-file atomicity, conflict
prevention or crash durability. Cancellation cannot undo a committed write or
arbitrary subprocess side effects. Report publication, durability and cleanup
evidence separately using the shared execution lifecycle contract.

Network checks remain under `sandbox::network` and transport policy: enforce
host/port/scheme rules, and reject metadata IPs, private ranges and localhost
unless explicitly authorized. Filesystem binding does not grant network access.

The default network allowlist for local model tests may include:

```text
127.0.0.1:8848
```

No broader localhost, LAN, metadata-service, or public internet access should
be implied by that local test allowlist.

## 11. Authority Context

`etas_host` defines action-grant, authority, and approval values, but it does
not infer or grant language authority. Enforcement belongs to the execution
engine or host controller using checked effect facts, deployment manifests,
trace-spec monitors, sandbox rules, and trace state.

```rust
pub struct AuthorityContext {
    pub grants: Vec<HostActionGrant>,
    pub approvals: Vec<ApprovalGrant>,
    pub sandbox: SandboxPolicy,
    pub policy: PolicyContext,
}

pub enum HostActionGrant {
    Allow(ActionPattern),
}

pub struct ActionInstance {
    pub effect: String,
    pub action: String,
    pub args: Vec<HostValue>,
}

pub struct ApprovalRequest {
    pub id: HostRequestId,
    pub reason: String,
    pub requested_actions: Vec<ActionInstance>,
    pub trace: TraceContext,
}

pub enum ApprovalDecision {
    Approved { grant: ApprovalGrant },
    Denied { reason: String },
}
```

The interpreter may reject runtime-required authority contexts in Phase 1 when
it cannot mediate the requested action safely. The runtime may enforce the same
values through its scheduler and host controller. Both use the same protocol
values.

## 12. Trace And Budget

Trace and budget values should also be shared because model, tool, and memory
adapters need to preserve them independent of the engine that produced the
request.

```rust
pub struct TraceContext {
    pub trace_id: TraceId,
    pub parent_span: Option<TraceSpanId>,
}

pub enum TraceEvent {
    HostRequestStarted {
        id: HostRequestId,
        kind: HostRequestKind,
        authority: AuthorityContext,
    },
    HostRequestFinished {
        id: HostRequestId,
        outcome: HostOutcome,
    },
    ApprovalRequested {
        request: ApprovalRequest,
    },
}

pub struct Budget {
    pub tokens: Option<TokenBudget>,
    pub time: Option<TimeBudget>,
    pub cost: Option<CostBudget>,
}
```

`HostRequestKind` should include model, tool, typed memory, console/std-stream,
approval, filesystem, network, command, and checkpoint-related host boundaries
so traces can distinguish externally visible operations without inspecting
engine-local state.

`etas_host` owns shared execution-budget accounting and lifecycle mechanisms.
The engine applies language budget semantics, and checkpoint codecs preserve
durable consumption without persisting live cancellation state. Runtime policy
and application configuration choose limits; child scopes cannot widen them.

Extend trace records with scope/parent identity, cancellation requested and
observed, correlated operation evidence, and termination/cleanup state. Reuse
existing request/trace identities and redaction. A request can be confirmed
successful even when its enclosing run is cancelled. Cancellation must not add
a synthetic action to the compiler's requested-action summary.

## 13. Relationship To Other Core Crates

```text
etas_std
  declares standard host-facing names, effects, signatures, and intrinsic ids

etas_builtin
  executes pure deterministic intrinsic kernels

etas_host
  defines host protocol values, reusable provider/tool/memory adapters, and
  reusable workspace/sandbox and execution-lifecycle mechanisms
```

Do not mix these responsibilities:

- `etas_std` does not call OpenAI, MCP, files, network, or tools.
- `etas_builtin` does not execute authority-bearing behavior.
- `etas_host` does not lower HIR/AIR or execute Etas control flow.

## 14. Testing Direction

`etas_host` tests should cover protocol behavior without depending on a real
execution engine:

- `HostValue` roundtrip through schemas/codecs where a local test value model is
  sufficient;
- model request to provider-request encoding;
- provider-response to `ModelResponse` decoding;
- tool request argument encoding and response decoding;
- memory request/response encoding, version metadata, and conflict behavior;
- authority context preservation;
- trace id and request id preservation;
- rendering-neutral host error construction.
- workspace binding, root replacement and handle-relative path confinement;
- symlink races and descriptor inheritance rejection;
- atomic publication and explicitly staged rollback behavior;
- destructive operation denial by default;
- network allowlist denial by default;
- command execution denial by default;
- snapshot/diff audit output.

The lifecycle acceptance matrix in [Shared Execution Lifecycle](etas-execution-design.md#8-implementation-and-acceptance)
is required in addition to protocol tests, including actual cancellation races,
managed blocking work, partial/unknown external outcomes and pending cleanup.
The [Workspace acceptance matrix](etas-workspace-design.md#7-acceptance-matrix)
also requires real filesystem and child-process evidence for each advertised
platform guarantee, including duplicate registration, reopen, rename races,
isolation activation failure and the absence of ambient/unrestricted fallbacks.
The [storage acceptance matrix](etas-storage-design.md#10-acceptance-matrix)
adds database concurrency, commit certainty, allocation limits and source-level
version/receipt coverage. These are durable-state tests, not cache-miss tests.

Default tests should also cover concrete client request construction and
response decoding using a local fake transport. They should not require network
access.

Live provider tests should be opt-in integration tests, not default unit tests.
For local model connectivity, use the local omlx server:

```text
OpenAI-compatible base URL:    http://127.0.0.1:8848/v1
Anthropic-compatible base URL: http://127.0.0.1:8848
small model:                  Qwen3.5-0.8B-MLX-4bit
```

Live tests must:

- be ignored or feature/flag-gated by default;
- require an explicit environment variable such as `ETAS_HOST_LIVE_OMLX=1`;
- call the concrete `ModelClient::complete` implementation;
- send an actual small completion request, not only a socket probe;
- verify a non-empty assistant response or a structured provider error mapped
  into `HostError`;
- use short timeouts suitable for local development.

Sandbox tests must be strict and local-only:

- create temporary workspace roots under the test temp directory;
- never write to the user's home directory or repository outside the temporary
  workspace;
- test `../`, absolute paths, repeated separators, unicode-looking path
  variants, and symlinks pointing outside the workspace;
- test writes through symlinked directories;
- test delete/move operations against workspace and non-workspace targets;
- test that command execution is denied unless explicitly configured;
- test that network access is denied unless a host/port is explicitly
  allowlisted;
- use property or fuzz-style tests for path normalization when practical;
- assert that failed operations leave no committed filesystem changes outside
  the temporary workspace.
