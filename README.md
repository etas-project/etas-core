# Etas Core

Shared types, diagnostics, standard-library declarations, and host interfaces
for the Etas toolchain.

This is the common foundation used by the compiler, interpreter, and editor
tooling, not a standalone language executable. It includes:

- Source locations, diagnostics, identifiers, and shared data structures.
- Standard-library API declarations and pure builtin implementations.
- Host interfaces for observable operations such as I/O and model calls.
- Cache infrastructure and package-metadata contracts.

Standard-library declarations describe the source-facing API; pure builtins
implement deterministic operations; host adapters provide external services.
Keeping these separate lets different execution backends reuse the same API.

Experimental. To write and run Etas programs, start with the
[Etas CLI](https://github.com/etas-project/etas#quickstart).

## Bootstrap

Requires Rust 1.85+ and a native C/C++ build toolchain.

```sh
git clone https://github.com/etas-project/etas-core.git
cd etas-core
cargo build --workspace
```

This builds the shared Rust libraries. For a focused test run:

```sh
cargo test -p etas_core
```

## Documentation

- [Core design](docs/architect/phase1-core-design.md)
- [Repository boundaries](docs/architect/repository-boundary.md)

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)
