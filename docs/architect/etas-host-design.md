# Etas Host Design

Status: `Draft`

Owner: `Architect`

Last updated: `2026-07-02`

## 1. Purpose

`etas_host` is the shared crate for host-facing protocol values and reusable
host adapters.

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
Provider adapter is shared.
Engine adapter is not shared.
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
- workspace boundaries, path normalization, snapshots, diffs, and rollback
  primitives;
- sandbox policy values and reusable sandbox brokers for filesystem, network,
  and command execution;
- action grant and authority context values;
- approval request and decision values;
- console/stdin/stdout/stderr request values and reusable client interfaces;
- trace context and host trace events;
- budget values for tokens, time, and cost;
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
- runtime scheduling;
- checkpoint/resume implementation;
- continuation machinery;
- language-level trace-spec decisions, approval decisions, or grant derivation;
- provider configuration discovery policy;
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

    context/
      mod.rs
      request.rs
      authority.rs
      budget.rs
      trace.rs
      error.rs

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
      sqlite.rs
      postgres.rs
      vector.rs

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
      sqlite.rs
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
      workspace.rs
      filesystem.rs
      network.rs
      command.rs
      snapshot.rs
      diff.rs
      platform.rs

    testing/
      mod.rs
      fake_transport.rs
      fake_sandbox.rs
      fixtures.rs
      assertions.rs
```

Layering:

- `value` defines shared wire-facing values and schemas.
- `context` defines request ids, action-grant context, approval values, trace
  context, budget values, and rendering-neutral errors.
- `transport` defines reusable HTTP/SSE/auth/timeout/retry mechanics used by
  provider and tool clients, including network allowlist checks.
- `model` defines model protocols and model provider clients.
- `tool` defines tool protocols and reusable tool clients.
- `memory` defines typed persistent-memory protocols and reusable backend
  clients.
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
  plumbing, not continuation or scheduler ownership.
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
  filesystem, network, and command execution.
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
models memory through ordinary types such as `MemoryRegion[S]` and
`Store[K, V]`, then binds immutable resource handles with top-level `let`.
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

The request protocol should preserve region, store, version, authority, trace,
and budget information:

```rust
pub struct MemoryRegionRef {
    pub stable_id: String,
    pub schema_fingerprint: Option<String>,
}

pub struct StoreRef {
    pub region: MemoryRegionRef,
    pub path: Vec<String>,
}

pub struct MemoryRequest {
    pub id: HostRequestId,
    pub store: StoreRef,
    pub operation: MemoryOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

pub enum MemoryOperation {
    Get {
        key: HostValue,
    },
    Put {
        key: HostValue,
        value: HostValue,
        expected: Option<MemoryVersion>,
    },
    Delete {
        key: HostValue,
        expected: Option<MemoryVersion>,
    },
    Scan {
        cursor: Option<MemoryCursor>,
        limit: Option<u32>,
    },
    Query {
        query: MemoryQuery,
        limit: Option<u32>,
    },
    VectorSearch {
        embedding: Vec<f32>,
        limit: u32,
        filter: Option<HostValue>,
    },
}

pub struct MemoryResponse {
    pub id: HostRequestId,
    pub result: Result<MemoryResult, HostError>,
}

pub enum MemoryResult {
    None,
    Value {
        value: HostValue,
        version: MemoryVersion,
    },
    Entries {
        entries: Vec<MemoryEntry>,
        cursor: Option<MemoryCursor>,
    },
    Written {
        version: MemoryVersion,
    },
    Deleted {
        version: MemoryVersion,
    },
    Conflict(MemoryConflict),
}

pub struct MemoryEntry {
    pub key: HostValue,
    pub value: HostValue,
    pub version: MemoryVersion,
}

pub struct MemoryVersion {
    pub opaque: String,
}

pub struct MemoryConflict {
    pub expected: Option<MemoryVersion>,
    pub actual: Option<MemoryVersion>,
    pub current_value: Option<HostValue>,
}

pub struct MemoryCursor {
    pub opaque: String,
}
```

`MemoryQuery` should be a host protocol query shape, not an Etas expression
tree. It may support a conservative portable subset first:

```rust
pub struct MemoryQuery {
    pub predicate: Option<HostValue>,
    pub order_by: Vec<MemoryOrderKey>,
}

pub struct MemoryOrderKey {
    pub field_path: Vec<String>,
    pub descending: bool,
}
```

Backends can expose richer backend features later through explicit feature
descriptors. The shared protocol should not assume every backend supports
full SQL, vector search, transactions, or secondary indexes.

Memory clients implement one reusable trait:

```rust
pub trait MemoryClient {
    type Error;

    async fn execute(
        &self,
        request: MemoryRequest,
    ) -> Result<MemoryResponse, Self::Error>;
}
```

Reusable backend adapters can live under `etas_host::memory` where the
protocol is generic enough:

```text
memory::sqlite::SqliteMemoryClient
  useful for local development, tests, and single-user prototypes
  maps StoreRef to tables or namespaced key-value tables
  preserves MemoryVersion with row/version metadata

memory::postgres::PostgresMemoryClient
  useful for server deployments
  maps StoreRef to schemas/tables or configured relation bindings
  preserves MemoryVersion with MVCC/version columns or backend-specific tokens

memory::vector::VectorMemoryClient
  useful for retrieval memory
  supports VectorSearch and metadata filters where configured
  still preserves StoreRef, trace, authority, and budget
```

Backend adapters may translate `MemoryRequest` to SQL, key-value operations, or
vector-store APIs. They must not:

- depend on HIR or AIR;
- decide language-level authority;
- infer `Memory.read[R]` / `Memory.write[R]` actions;
- know interpreter frames or runtime scheduler state;
- render diagnostics;
- silently ignore version preconditions.

The execution engine or host controller supplies `AuthorityContext` with checked
action grants and policy/sandbox context. The memory client must preserve
request ids, trace context, budget context, and rendering-neutral errors.
Version conflicts should be returned as structured `MemoryConflict` results or
mapped to `HostError` with enough detail for the engine to decide whether to
retry, resume, or report a conflict.

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
Memory.read[ProjectMemory]
Memory.read[ProjectMemory.Papers]
Memory.write[ProjectMemory.Drafts]
```

`etas_host` defines reusable request/grant values. The interpreter/runtime
decide how checked effect facts, deployment manifests, approval records, and
active policies produce the grants.

### 8.2 Memory Tests

Default memory tests should be deterministic and local:

- fake `MemoryClient` request/response roundtrips;
- request id, authority, trace, and budget preservation;
- `HostValue` key/value encoding;
- version precondition success and conflict cases;
- denied authority mapped to `HostError`;
- sqlite adapter tests using temporary database files only;
- vector adapter protocol tests with fake transport or in-memory fixtures.

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
    ReadBytes {
        path: WorkspacePath,
    },
    WriteBytes {
        path: WorkspacePath,
        body: Vec<u8>,
        atomic: bool,
    },
    List {
        path: WorkspacePath,
    },
    Stat {
        path: WorkspacePath,
    },
    AtomicReplace {
        path: WorkspacePath,
        body: Vec<u8>,
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

Filesystem substrate must pass through workspace canonicalization and escape
checks before any host filesystem access. Network and browser substrate must
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
- path canonicalization, symlink escape, atomic write, and rollback behavior for
  `std.fs`;
- secret redaction in debug output, trace payloads, and checkpoint-like
  snapshots;
- browser session/origin binding and denial of unapproved profile/session use;
- fake clients for deterministic unit tests and opt-in live tests only when a
  local or explicitly configured service is available.

## 10. Sandbox And Workspace

`etas_host` should provide default sandbox and workspace building blocks
because model, tool, and memory-backend execution are among the highest-risk
host boundaries. The default posture must be deny-by-default and
workspace-scoped.

The split is:

```text
etas_host owns:
  WorkspaceRoot
  WorkspacePath
  WorkspaceSnapshot
  WorkspaceDiff
  SandboxPolicy
  SandboxBroker
  filesystem/network/command sandbox adapters
  path canonicalization and escape checks
  reusable test fakes and assertions

interpreter/runtime own:
  deriving action grants from checked program facts and deployment manifests
  deciding whether approval is required
  deciding whether a request may proceed
  deciding whether to commit or roll back produced changes
  mapping sandbox failures into language-level diagnostics or runtime failures
```

`etas_host` therefore provides runtime admission mechanics for supplied
trace-spec/admission context and sandbox profile, but it does not decide
language authority. The caller chooses active trace specs, action grants,
approval records, and sandbox profiles; `etas_host` performs low-level safety
checks and returns rendering-neutral results.

Recommended sandbox types:

```rust
pub struct WorkspaceRoot {
    pub canonical_root: PathBuf,
}

pub struct WorkspacePath {
    pub root: WorkspaceRoot,
    pub relative: PathBuf,
}

pub struct SandboxPolicy {
    pub filesystem: FilesystemPolicy,
    pub network: NetworkPolicy,
    pub command: CommandPolicy,
    pub destructive_ops: DestructiveOpPolicy,
}

pub struct SandboxBroker {
    // Owns configured policies and delegates to filesystem/network/command
    // sandbox implementations.
}
```

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

Workspace operations should be designed around snapshots and diffs:

```text
begin snapshot
  run tool/model-side workspace operation
  collect writes as staged changes
  produce diff
commit only if caller accepts
rollback otherwise
```

`etas_host` should support safe workspace primitives:

```text
sandbox::workspace
  canonicalize root
  resolve relative paths
  reject absolute paths outside root
  reject `..` traversal
  reject symlink escapes

sandbox::filesystem
  read file within workspace
  atomic write within workspace
  create directory within workspace
  delete/move-to-trash within workspace policy

sandbox::network
  check host/port/scheme allowlist
  reject metadata IPs, private ranges, and localhost unless explicitly allowed
  integrate with transport::network_policy

sandbox::command
  deny by default
  support allowlisted commands only
  provide hooks for platform sandboxes such as Landlock, container execution,
  or WASI-style preopened directories when available

sandbox::snapshot / sandbox::diff
  compute staged file changes
  expose readable audit diffs
  support rollback on failure or rejection
```

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

`etas_host` defines the values. Runtime policy decides how budgets are
reserved, consumed, exceeded, reported, or checkpointed.

## 13. Relationship To Other Core Crates

```text
etas_std
  declares standard host-facing names, effects, signatures, and intrinsic ids

etas_builtin
  executes pure deterministic intrinsic kernels

etas_host
  defines host protocol values, reusable provider/tool/memory adapters, and
  reusable workspace/sandbox safety mechanics
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
- workspace path canonicalization and escape rejection;
- symlink escape rejection;
- atomic write and rollback behavior;
- destructive operation denial by default;
- network allowlist denial by default;
- command execution denial by default;
- snapshot/diff audit output.

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
