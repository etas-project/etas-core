# Etas Core Repository Boundary

## Responsibility

`etas-core` owns shared infrastructure, shared language contracts, and
reusable pure builtin kernels:

- source files, spans, line indexes, diagnostics;
- ids, arenas, interners, result helpers;
- generic `fixpoint`, `graph`, and pass-pipeline utilities;
- generic artifact cache, fingerprint, dependency, and invalidation primitives;
- standard-library declarations and intrinsic descriptors;
- pure deterministic builtin implementations shared by interpreter and
  runtime;
- shared host protocol values and reusable host protocol adapters for model,
  tool, typed memory, console, authority context, trace, and budget boundaries;
- semantics-light cross-repository utility contract types.

Phase 1 crate-level design is recorded in
`docs/architect/phase1-core-design.md`.

## Forbidden

This repository must not own:

- parser or AST semantics;
- HIR, type checking, effect checking;
- AIR builders, AIR verifiers, FIR, or optimization;
- runtime execution, runtime scheduling, checkpointing, continuations, or AIR
  dispatch;
- engine-specific conversion from HIR/AIR values into host requests;
- authority enforcement decisions, approval workflow execution, or sandbox
  policy execution;
- authority-bearing builtins such as console/std-stream, filesystem, network,
  time, model, tool, approval, checkpoint, or memory execution;
- LSP protocol handling or editor extensions.
- frontend-specific artifact semantics or pass invalidation rules.

## Dependency Rule

`etas-core` has no dependency on other Etas repositories.

`etas_std` is already an `etas-core` crate and remains the declaration-only
standard language support surface. Incremental compiler storage should be added
as `etas_cache`, not by moving standard-library responsibilities into frontend
or IDE crates.
