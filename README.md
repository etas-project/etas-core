# Etas Core

Shared, dependency-bottom infrastructure for the Etas toolchain.

`etas-core` owns stable data contracts and reusable mechanisms used by the
frontend, interpreter, CLI, package tooling, IDE, and future AIR runtime. It
does not own source parsing, HIR semantics, type/effect checking, interpreter
control flow, package resolution, or AIR/FIR lowering.

## Workspace Crates

| Crate | Responsibility |
|---|---|
| `etas_core` | Source files, spans, diagnostics, typed IDs, arenas, interning, and common result types |
| `etas_utils` | Pipelines, fixpoints, abstract domains, graph algorithms, automata, reusable patterns, and profiling |
| `etas_cache` | Artifact identities, fingerprints, dependency records, SQLite/WAL storage, blob compression, invalidation, and telemetry |
| `etas_package_metadata` | Versioned public package metadata and compressed binary artifact encoding |
| `etas_std` | Standard-package module, type, flow, effect, action, intrinsic, and editor declarations |
| `etas_builtin` | Runtime-neutral implementations of pure deterministic intrinsics |
| `etas_host` | Explicit host protocols and adapters for console, filesystem, commands, network, TLS, streams, memory, sessions, tools, models, approvals, secrets, and browsers |

The boundaries between the last three crates are intentional:

```text
etas_std       declares the source-visible API and its static metadata
etas_builtin   executes pure, runtime-neutral intrinsic kernels
etas_host      mediates observable host operations and authority
```

A host operation must not be hidden in `etas_builtin`, and `etas_host` must not
invent source-language semantics. Runtime authorization policy is an execution
concern carried through host requests; it is not the removed source-level
`policy` language construct.

## Dependency Boundary

This repository is the bottom of the Etas dependency graph:

```text
etas-core
   ^
   +-- etas-frontend
   +-- etas-interpreter
   +-- etas
   +-- etas-runtime
   +-- etas-optimizing
   +-- etas-ide
```

It must not depend on another Etas repository. Crates inside this workspace may
depend on one another in the direction encoded by Cargo, but public contracts
must remain independent of frontend and execution implementation details.

Shared mechanisms belong here only when at least two higher layers can use
them without importing layer-specific state. For example, a generic fixpoint
engine belongs in `etas_utils`; an effect-domain transfer function remains in
`etas_effects`.

## Host Boundary

Every observable host request carries stable request identity, authority,
trace context, budget/cancellation data, and a typed operation payload. Concrete
adapters must fail closed when a service or grant is absent. They must not
return empty values, mock success, or private package-specific fallbacks.

Filesystem access is workspace-scoped and canonicalized. Network, browser,
command, secret, memory, model, and tool operations remain explicit service
families so the interpreter and future runtime can share protocol contracts
without sharing their execution engines.

## Cache Boundary

`etas_cache` is a generic, cross-process artifact store, not a live compiler
session. SQLite metadata uses WAL and transactions; immutable blobs use atomic
writes and content fingerprints. Frontend sessions decide what is worth
persisting and which artifacts should be recomputed.

## Build and Verify

The workspace requires Rust `1.85` or newer.

```bash
cargo build --workspace
```

Standard verification:

```bash
cargo fmt --all -- --check
cargo test --workspace --offline
cargo clippy --workspace --offline -- -D warnings
```

Focused crates can be checked independently:

```bash
cargo test -p etas_utils --offline
cargo test -p etas_cache --offline
cargo test -p etas_std --offline
cargo test -p etas_host --offline
```

`--offline` assumes Cargo dependencies have already been fetched.

## Downstream Use

Committed downstream dependencies use the repository Git identity. During
multi-repository development, the top-level `etas` checkout supplies local
Cargo patches for sibling repositories; do not commit developer-specific paths
inside this repository.

The language specification and user-facing CLI live in
[`etas`](https://github.com/etas-project/etas).

## Architecture Documents

- [Phase 1 core architecture](docs/architect/phase1-core-design.md)
- [Utilities and analysis infrastructure](docs/architect/etas-utils-design.md)
- [Artifact cache](docs/architect/etas-cache-design.md)
- [Pure builtin kernels](docs/architect/etas-builtin-design.md)
- [Host services](docs/architect/etas-host-design.md)
- [Repository boundary](docs/architect/repository-boundary.md)

## License

Etas Core is distributed under the terms of both the
[MIT License](LICENSE-MIT) and the
[Apache License (Version 2.0)](LICENSE-APACHE). You may choose either license.
