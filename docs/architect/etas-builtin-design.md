# Etas Builtin Design

Status: `Draft`

Owner: `Architect`

Last updated: `2026-06-21`

## 1. Purpose

`etas_builtin` is the shared implementation crate for pure deterministic
intrinsic kernels described by `etas_std`.

It exists so the Phase 1 HIR interpreter and the future AIR runtime do not
duplicate primitive arithmetic, text, bytes, collection, option/result, assert,
abort, pure JSON behavior, deterministic codecs, and deterministic crypto
helpers.

`etas_builtin` is not the standard library declaration layer and not an
execution engine:

```text
etas_std      = declares standard types, functions, effects, intrinsic
                 descriptors, intrinsic ids, and docs
etas_builtin  = implements pure deterministic intrinsic kernels
interpreter    = adapts HIR values and executes checked HIR
runtime        = adapts runtime values and executes AIR with authority checks
```

## 2. Crate Position

```text
etas-core/
  crates/
    etas_core/
    etas_std/
    etas_builtin/
```

Dependency direction:

```text
etas_builtin -> etas_core
etas_builtin -> etas_std

etas-interpreter -> etas_builtin
etas-runtime     -> etas_builtin
```

`etas_builtin` must never depend on `etas-frontend`, `etas-interpreter`,
`etas-runtime`, `etas-optimizing`, `etas-ide`, or `etas`.

## 3. Ownership

`etas_builtin` owns:

- pure arithmetic and comparison for concrete primitive widths;
- deterministic numeric conversion/checking helpers allowed by the PL design;
- text and `char` helpers, including length, trim/case helpers, line/split/join
  helpers, and concrete primitive parse/string conversion helpers;
- byte sequence helpers;
- `List`, `Map`, and `Set` pure helpers, including list length/emptiness,
  string-list joining, and map key-membership helpers such as `contains_key`;
- `Option` and `Result` pure helpers;
- `assert` and `abort` kernel behavior;
- pure JSON parse/stringify helpers if they are defined as deterministic
  builtins;
- deterministic codec helpers such as UTF-8 encode/decode, charset dispatch,
  base64/hex helpers, and HTTP framing helpers when declared pure by
  `etas_std`;
- deterministic crypto helpers such as hashes, HMAC, digest encoding, and
  constant-time comparison when declared pure by `etas_std`;
- builtin-local error taxonomy.

It does not own:

- HIR expression or statement evaluation;
- AIR scheduling or effect handling;
- interpreter frames, closures, local slots, or control signals;
- runtime trace, checkpoint, budget, approval, authority, or host adapters;
- filesystem, network, provider, model, tool, memory, clock, randomness, or
  sandbox execution;
- secret retrieval, secret-store access, TLS session state, certificate
  validation, secure randomness, browser sessions, or stream/file handles;
- CLI diagnostic rendering.

## 4. Internal Layout

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

    codec/
      mod.rs
      text.rs
      binary.rs
      http.rs

    crypto/
      mod.rs
      hash.rs
      hmac.rs
      digest.rs
      constant_time.rs

    control/
      mod.rs
      assert.rs
      abort.rs

    error.rs
```

Layering:

- `dispatch` maps `StdIntrinsicId` to a pure kernel.
- `value` defines the builtin boundary value and adapter trait.
- domain folders implement kernels without knowing about HIR or AIR.
- `error` defines structured failure cases.

## 5. Value Boundary

`etas_builtin` should not take interpreter values or runtime values directly.
It should expose a small builtin-local boundary value:

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
```

Rust enum variants use Rust naming conventions. Source-facing diagnostics and
dumps must render PL names exactly: `bool`, `i32`, `f64`, `string`, `bytes`,
`unit`, and so on.

`BuiltinValue` must preserve Etas collection semantics. `Array[T]` and
`List[T]` are distinct source types even if an adapter stores both as Rust
vectors internally. Pure builtin kernels may accept both only through an
explicitly declared overload or shared helper; they must not silently coerce one
family into the other. `Range` stores bound values and inclusivity explicitly,
and `Slice` exposes value semantics to pure builtin code.

Interpreter/runtime integration should use adapters:

```rust
pub trait BuiltinValueAdapter {
    type Value;
    type Error;

    fn into_builtin(value: Self::Value) -> Result<BuiltinValue, Self::Error>;
    fn from_builtin(value: BuiltinValue) -> Result<Self::Value, Self::Error>;
}
```

This keeps HIR-only values and runtime-only values out of `etas_builtin`.

## 6. Dispatch API

Recommended API shape:

```rust
pub fn call_pure_intrinsic(
    intrinsic: StdIntrinsicId,
    args: &[BuiltinValue],
) -> Result<BuiltinValue, BuiltinError>;
```

Dispatch should be by `StdIntrinsicId`, not by strings.

`etas_std` declares whether an intrinsic is pure, runtime-backed, or
host-backed. `etas_builtin` should execute only pure intrinsics. Non-pure ids
should return a structured unsupported error so interpreter/runtime layers can
emit the correct diagnostic.

Pure standard substrate helpers may be builtin kernels:

```text
std.http.codec.encode_request
std.http.codec.decode_response_head
std.codec.text.utf8_decode
std.codec.text.utf8_encode
std.crypto.hmac_sha256
std.crypto.constant_time_eq
```

Host-bearing substrate operations must never be builtin kernels:

```text
std.net.tcp.connect
std.stream.read
std.stream.write_all
std.tls.connect
std.fs.read_bytes
std.fs.write_bytes
std.secret.read
std.browser.protocol.send
```

Secret-bearing inputs may be passed through adapters as redaction-aware builtin
values, but `etas_builtin` must not retrieve secrets and must not render secret
contents in errors.

## 7. Error Model

`BuiltinError` should be structured and rendering-neutral:

```rust
pub enum BuiltinError {
    UnsupportedIntrinsic { intrinsic: StdIntrinsicId },
    ArityMismatch { expected: usize, actual: usize },
    TypeMismatch { expected: BuiltinTypeTag, actual: BuiltinTypeTag },
    NumericOverflow,
    DivideByZero,
    InvalidShift,
    InvalidSlice,
    InvalidUtf8,
    InvalidJson,
    AssertionFailed,
    Abort { message: String },
}
```

`etas_builtin` should not directly render diagnostics. The HIR interpreter and
AIR runtime map `BuiltinError` into their own diagnostic or runtime failure
contexts, preserving source spans when available.

## 8. Runtime Boundary

Some standard declarations look like ordinary functions but require runtime
authority:

- model/provider calls;
- tool calls;
- memory reads/writes;
- filesystem/network/command operations;
- clock/time reads;
- approval;
- checkpoint/resume;
- trace emission;
- budget accounting.

These must not be implemented in `etas_builtin`. Shared protocol values and
reusable provider/tool protocol adapters belong in `etas_host`; actual
execution policy, authority enforcement, scheduling, checkpointing, and runtime
state belong in the interpreter or AIR runtime using that protocol.
`etas_builtin` may only report that such an intrinsic is unsupported by the
pure builtin layer.

## 9. Testing Direction

`etas_builtin` tests should be pure Rust tests with deterministic inputs:

- numeric arithmetic and overflow behavior;
- concrete-width conversion behavior;
- string and char operations;
- bytes operations;
- list/map/set helper behavior;
- option/result helper behavior;
- assert/abort behavior;
- JSON parse/stringify behavior if enabled;
- dispatch by `StdIntrinsicId`;
- unsupported non-pure intrinsic diagnostics.

Tests should not load Etas source fixtures. Parser, HIR, and CLI integration
tests belong in downstream repositories.
