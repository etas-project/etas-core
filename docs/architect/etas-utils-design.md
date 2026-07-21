# Etas Utils Design

## 1. Purpose

This document defines the architecture of `etas_utils`.

`etas_utils` is the generic algorithm utility crate for Etas. It is broader
than a pure fixpoint crate, but it must remain narrower than a `common` or
miscellaneous helper crate.

It exists for reusable, semantics-free algorithms used by type checking, effect
checking, AIR utilities, and static analysis.

## 2. Boundary

`etas_utils` owns:

- generic lattice traits;
- transfer and constraint traits;
- worklist data structures;
- fixpoint iteration engines;
- generic graph traits and adapters;
- graph traversal algorithms;
- topological sorting;
- strongly connected components;
- reverse postorder utilities;
- generic dominator algorithms;
- generic pass pipeline orchestration;
- simple behavior patterns such as responsibility chain.

`etas_utils` does not own:

- source files, spans, diagnostics, ids, arenas, or interners;
- AST, HIR, AIR, type, effect, policy, or runtime data models;
- CFG data models;
- DFG data models;
- effect graphs;
- AIR graph semantic definitions;
- compiler, optimizer, interpreter, or runtime pass semantics;
- type environments;
- effect rows;
- taint domains;
- approval domains;
- budget domains;
- prompt or memory analysis domains.

In short: `etas_utils` may provide generic algorithms and orchestration
patterns, but it must not define Etas's semantic graphs or pass semantics.

## 3. Crate Layout

Recommended file layout:

```text
crates/etas_utils/
  Cargo.toml
  src/
    lib.rs

    fixpoint/
      mod.rs
      lattice.rs
      transfer.rs
      worklist.rs
      solver.rs
      iteration.rs

    graph/
      mod.rs
      traversal.rs
      topo.rs
      scc.rs
      dominator.rs

    pipeline/
      mod.rs
      pass.rs
      unit.rs
      adapter.rs
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
      mod.rs
      responsibility_chain.rs
```

Module responsibilities:

| Module | Responsibility |
|---|---|
| `fixpoint` | Generic fixpoint solving framework |
| `fixpoint::lattice` | Partial order, join, meet, top, bottom, and widening traits |
| `fixpoint::transfer` | Generic transfer, edge-transfer, and constraint traits |
| `fixpoint::worklist` | Reusable worklist queues and scheduling strategies |
| `fixpoint::solver` | Generic fixpoint engine and result model |
| `fixpoint::iteration` | Iteration limits, convergence status, and solver diagnostics |
| `graph` | Semantics-free graph traits and graph algorithms |
| `pipeline` | Generic pass manager, pass-local analysis preservation/invalidation, instrumentation, and pipeline orchestration over caller-owned contexts |
| `pattern` | Small reusable design-pattern helpers such as responsibility chain |

## 4. Relationship To `etas_core`

`etas_core` holds primitives:

```text
SourceId
SourceFile
Span
Diagnostic
Arena
Interner
```

`etas_utils` holds algorithms:

```text
Lattice
Transfer
Worklist
FixpointEngine
GraphView
SCC
TopologicalSort
DominatorTree
Pass
Pipeline
ResponsibilityChain
```

This separation keeps `etas_core` small and keeps generic algorithms out of
semantic crates.

## 5. Fixpoint Framework

The fixpoint framework should be generic over state, nodes, and transfer
functions.

Illustrative shape:

```rust
pub trait PartialOrder {
    fn less_equal(&self, other: &Self) -> bool;
}

pub trait JoinSemiLattice: PartialOrder {
    fn bottom() -> Self;
    fn join_assign(&mut self, other: &Self) -> bool;
}

pub trait Transfer<State> {
    fn apply(&self, state: &mut State) -> bool;
}
```

The exact API can evolve, but it should not mention Etas-specific concepts.

Allowed users:

- `etas_types` for type narrowing, subtype closure, or constraint propagation;
- `etas_effects` for effect and requested-action propagation;
- `etas_analysis` for taint, approval dominance, budget, and memory analyses;
- `etas_air` for generic graph utilities over AIR-owned graph structures.

The framework should not replace domain-specific solvers where they are a
better fit. For example, unification, occurs checks, and type substitutions
remain in `etas_types`.

## 6. Graph Algorithms

`etas_utils::graph` should expose algorithms over generic graph views.

Useful algorithms:

- DFS and BFS traversal;
- reverse postorder;
- topological sorting;
- cycle detection;
- strongly connected components;
- reachability;
- dominator tree;
- post-dominator tree later if needed.

The graph layer should work over caller-owned graph models. For example:

```text
etas_air owns AirGraph
etas_analysis owns Cfg and Dfg views
etas_utils owns algorithms over graph-like inputs
```

`etas_utils` should not know whether a node is an AIR node, HIR expression,
basic block, effect action, or runtime step.

## 7. Semantic Graph Boundary

These belong outside `etas_utils`:

```text
AIR graph data model        -> etas_air
CFG construction            -> etas_analysis
DFG construction            -> etas_analysis
effect graph construction   -> etas_effects or etas_analysis
call graph construction     -> etas_analysis
policy graph construction   -> etas_analysis
```

These may live in `etas_utils`:

```text
generic graph traits
generic graph traversal
SCC algorithm
topological sort
dominator algorithm
worklist scheduler
fixpoint engine
```

This distinction is important. A generic SCC algorithm is reusable
infrastructure; an effect graph is Etas semantics.

## 8. Pass Pipeline Pattern

`etas_utils::pipeline` should provide a generic pass manager pattern reusable
by the frontend, optimizing middle-end, interpreter planning, runtime preflight,
IDE analysis refresh, and tests.

The pass pipeline is orchestration infrastructure. It should not know what a
HIR node, type fact, effect row, FIR node, AIR node, runtime task, host adapter,
or interpreter frame means.

Passes should represent compiler, interpreter, runtime, or IDE phases with
clear input and output artifacts. They should not represent local grammar
branches or small parser details. For example, `ParsePass`, `HirLowerPass`,
`NameResolutionPass`, `TypeCheckPass`, and `EffectCheckPass` are valid pass
boundaries. `GroupedImportPass`, `GroupedAliasPass`, `TrailingCommaPass`, and
`WildcardImportSyntaxPass` are not valid pass boundaries; those are grammar
forms handled inside parsing or import lowering.

### 8.1 Relationship To Responsibility Chain

`pattern::responsibility_chain` is a small behavior pattern:

```rust
pub trait ChainStep<C> {
    fn run(&mut self, context: &mut C) -> ChainControl;
}
```

It is useful for simple ordered handlers. A compiler pass pipeline uses the
same idea of "caller-owned context plus ordered steps", but it needs a richer
contract:

- pass names for diagnostics and tracing;
- pass results with success, failure, and stop control;
- changed/no-change reporting;
- optional timing and stats;
- deterministic pass order;
- required/produced artifact declarations and invalidation.

Therefore `pipeline` should be a separate module, not an alias for
`ResponsibilityChain`.

### 8.2 Complete Pass Manager Contract

The pipeline API should be designed as a complete pass manager from the start.
Implementation can land pass by pass, but the public contract should expose
the complete pass-manager model from the beginning so later consumers do not
need an incompatible redesign.

Required capabilities:

- named passes and pass groups;
- nested pipelines;
- deterministic schedule order;
- pass result control: continue, stop, failed;
- changed/no-change reporting;
- required artifact declarations;
- produced artifact declarations;
- preserved artifact declarations;
- invalidation of stale pass-local analyses;
- analysis cache keyed by caller-owned pass analysis ids;
- pass instrumentation hooks;
- pass timing and stats;
- pass enable/disable filters for CLI debugging and tests;
- unit-scoped pass execution through generic adapters;
- scoped artifacts and invalidation;
- rendering-neutral failure reasons.

Recommended API shape:

```rust
pub trait Pass<C> {
    fn descriptor(&self) -> PassDescriptor;
    fn run(
        &mut self,
        context: &mut C,
        pass_context: &PassContext<C>,
        manager: &mut PassManager<C>,
    ) -> PassResult;
}

pub struct PassDescriptor {
    pub name: &'static str,
    pub kind: PassKind,
    pub scope: PassScope,
    pub requires: ArtifactSet,
    pub produces: ArtifactSet,
}

pub enum PassKind {
    Transform,
    Analysis,
    Verify,
    Plan,
    Emit,
}

pub enum PassScope {
    Global,
    Unit(UnitKindKey),
}

pub struct UnitKindKey {
    pub namespace: &'static str,
    pub name: &'static str,
}

pub struct UnitKey {
    pub kind: UnitKindKey,
    pub id: u64,
}

pub struct PassContext<C> {
    pub current_unit: Option<UnitKey>,
    marker: PhantomData<C>,
}

pub struct Pipeline<C> {
    pub name: &'static str,
    pub passes: Vec<PipelineStep<C>>,
}

pub enum PipelineStep<C> {
    Pass(Box<dyn Pass<C>>),
    Group(Pipeline<C>),
    ForEach(ForEachAdapter<C>),
}

pub struct ForEachAdapter<C> {
    pub selector: UnitSelector,
    pub order: UnitOrder,
    pub pipeline: Pipeline<C>,
}

pub enum UnitSelector {
    Kind(UnitKindKey),
    Affected(UnitKindKey),
}

pub enum UnitOrder {
    Stable,
    SourceOrder,
    DependencyOrder,
    ReverseDependencyOrder,
    AffectedFirst,
    Custom(&'static str),
}

pub struct PassManager<C> {
    analyses: AnalysisCache,
    instrumentation: Vec<Box<dyn PassInstrumentation<C>>>,
    config: PipelineConfig,
}

pub struct PassResult {
    pub control: PassControl,
    pub changed: bool,
    pub preserved: PreservedArtifacts,
    pub produced: ArtifactSet,
}

pub enum PassControl {
    Continue,
    Stop,
    Failed(PassFailure),
}
```

Generic boundary:

```text
Pipeline<C> owns pass ordering and grouping.
PassManager<C> owns pass orchestration metadata.
C owns all semantic state.
Pass<C> mutates caller-owned context.
AnalysisCache stores caller-defined analysis results behind generic keys.
```

`etas_utils::pipeline` may define generic ids and containers such as
`ArtifactKey`, `ArtifactSet`, `PreservedArtifacts`, `AnalysisCache`,
`UnitKindKey`, `UnitKey`, `UnitSelector`, and `ForEachAdapter`, but it must not
define concrete artifacts or units such as `HirProgram`, `TypeFacts`,
`EffectFacts`, `HirModuleId`, `HirBlockId`, `FirGraph`, `AirProgram`,
`EvalPlan`, or `TracePlan`.

### 8.3 Unit Adapters

The pipeline must support unit-scoped execution without knowing frontend,
optimizing, interpreter, or runtime semantics. The caller-owned context exposes
units through a generic provider contract:

```rust
pub trait UnitProvider {
    fn units(&self, selector: &UnitSelector, order: UnitOrder) -> Vec<UnitKey>;
    fn parent(&self, unit: UnitKey) -> Option<UnitKey>;
    fn children(&self, unit: UnitKey, kind: Option<UnitKindKey>) -> Vec<UnitKey>;
}
```

`ForEachAdapter` runs a child pipeline once for each unit returned by the
context:

```text
ForEach(Body, Stable)
  TypeCheckBodyPass
  EffectCheckBodyPass
```

The manager records each child pass with its current unit:

```text
TypeCheckBodyPass @ frontend.body#42
EffectCheckBodyPass @ frontend.body#42
TypeCheckBodyPass @ frontend.body#43
EffectCheckBodyPass @ frontend.body#43
```

This design keeps semantic traversal out of business passes. A `Body` pass
receives the selected unit and checks exactly that unit; it must not rediscover
all bodies by walking the whole HIR program.

Unit adapter rules:

- adapters are scheduling steps, not semantic passes;
- unit kinds are opaque names owned by the caller;
- unit ordering is requested by generic `UnitOrder`, but interpreted by the
  caller context;
- project/global passes run with `current_unit = None`;
- unit passes run with `current_unit = Some(UnitKey)`;
- instrumentation and timing records must include the current unit;
- pass filters may target a pass globally or a pass at a specific unit;
- contexts that do not need unit-scoped scheduling may expose only a root unit or
  avoid adapters entirely.

The adapter model is similar in spirit to LLVM pass-manager adaptors such as
module-to-function adaptors, but it remains language-neutral and caller-owned.

### 8.4 Artifacts, Analyses, And Invalidation

The pass manager should support LLVM-style analysis preservation in a generic
Etas form.

```rust
pub struct ArtifactKey {
    pub namespace: &'static str,
    pub name: &'static str,
}

pub enum ArtifactScope {
    Global,
    Unit(UnitKey),
    UnitKind(UnitKindKey),
}

pub struct ArtifactRef {
    pub key: ArtifactKey,
    pub scope: ArtifactScope,
}

pub struct ArtifactSet {
    pub artifacts: Vec<ArtifactRef>,
}

pub enum PreservedArtifacts {
    All,
    None,
    Some(ArtifactSet),
}

pub trait Analysis<C> {
    type Output: 'static;

    fn key(&self) -> ArtifactKey;
    fn run(&self, context: &C) -> Self::Output;
}
```

Expected behavior:

- transform passes declare which artifacts they preserve;
- the manager invalidates non-preserved cached analyses after a changed pass;
- unit-scoped passes invalidate unit-scoped artifacts before broader artifacts;
- verify and analysis passes usually preserve all artifacts;
- contexts define the concrete artifact names for their domain;
- stale facts must not survive silently after a mutating pass.

This cache is pass-local analysis preservation, not a cross-session compiler
artifact store. Reusable artifact keys, fingerprints, dependency graphs,
invalidation sets, memory stores, and disk artifact stores belong to
`etas_cache`. Domain-specific artifact meaning still belongs to the caller,
such as `etas_frontend`.

Example frontend artifact keys:

```text
frontend.parse_output
frontend.parsed_source
frontend.module_index
frontend.unit_tree
frontend.hir
frontend.symbol_table
frontend.type_facts
frontend.effect_facts
frontend.checked_program
```

Example optimizing artifact keys:

```text
optimizing.fir
optimizing.fir_dominators
optimizing.fir_effect_summary
optimizing.air
optimizing.air_verify_result
```

These are names, not semantic definitions. The owning crate defines the actual
data structures.

### 8.5 Instrumentation And Configuration

Instrumentation should be built into the design:

```rust
pub trait PassInstrumentation<C> {
    fn before_pass(&mut self, pass: &PassDescriptor, pass_context: &PassContext<C>, context: &C);
    fn after_pass(
        &mut self,
        pass: &PassDescriptor,
        pass_context: &PassContext<C>,
        result: &PassResult,
        context: &C,
    );
    fn after_invalidation(&mut self, invalidated: &ArtifactSet);
}
```

`PipelineConfig` should support:

- enabling/disabling passes by name;
- stopping before or after a pass for debugging;
- fail-fast versus collect-diagnostics behavior where the context supports it;
- timing collection;
- stats collection;
- deterministic text dump of the scheduled pipeline.

Diagnostics remain owned by caller contexts. The pipeline may report pass-level
failures, but syntax/type/effect/runtime diagnostics stay in their semantic
crates.

### 8.6 Pipeline Versus Fixpoint

Pass pipeline and fixpoint solving are different layers:

```text
Pass Pipeline = phase orchestration
Fixpoint      = recursive convergence inside a pass
```

For example:

```text
TypeCheckPass
  -> may use etas_utils::fixpoint internally

EffectCheckPass
  -> may use etas_utils::fixpoint internally

Pipeline
  -> only decides that TypeCheckPass runs before EffectCheckPass
```

The pipeline should not become a solver. Recursive type/effect/analysis
convergence remains inside the semantic pass that owns the domain.

### 8.7 Expected Users

Frontend pipeline:

```text
FrontendProjectContext
  BuildSourceSetPass
  ForEach(SourceFile)
    ParseSourceFilePass
  BuildModuleIndexPass
  BuildUnitTreePass
  BuildImportGraphPass
  DetectImportCyclesPass
  ComputeModuleTopoOrderPass
  PredeclareProjectSymbolsPass
  ForEach(ModulePart)
    NormalizeModuleImportsPass
    LowerModuleItemsPass
  ResolveImportsPass
  ResolvePathsPass
  BuildSignatureFactsPass
  ForEach(Body)
    TypeCheckBodyPass
    EffectCheckBodyPass
  VerifyInterpreterSupportPass
  ResolveEntryPointPass
  BuildCheckedProjectPass
```

Optimizing pipeline:

```text
OptimizingContext
  HirToFirPass
  FirVerifyPass
  FirAnalysisPass
  FirOptPass
  FirToAirPass
  AirVerifyPass
```

Interpreter planning pipeline:

```text
InterpreterPlanContext
  EvalCheckPass
  BuiltinSupportCheckPass
  LightAnalysisPass
  LightOptPass
  PlanBuildPass
```

Runtime preflight pipeline:

```text
RuntimePreflightContext
  AirVerifyPass
  AuthorityCheckPass
  HostBindingPass
  BudgetPlanPass
  TracePlanPass
```

The interpreter evaluator loop and runtime scheduler are not ordinary pass
pipelines. They may run preflight/planning pipelines before execution, but the
execution loop remains owned by `etas-interpreter` or `etas-runtime`.

### 8.8 Design Constraint

The pass manager should be powerful enough to avoid redesign when FIR/AIR
optimization, frontend incremental checking, runtime preflight, and interpreter
planning mature. The architecture should include the full pass-manager contract
now: nested pipelines, pass-local analysis cache, preservation, invalidation,
instrumentation, timing, stats, and pass filtering. Cross-session artifact cache
storage remains an `etas_cache` responsibility.

This does not mean `etas_utils` owns the implementation schedule for every
consumer. It means all consumers build against the same stable abstraction from
the beginning.

## 9. Dependency Rules

Allowed:

```text
etas_utils -> etas_core

etas_types    -> etas_utils
etas_effects  -> etas_utils
etas_air      -> etas_utils
etas_analysis -> etas_utils
etas_test     -> etas_utils
```

Forbidden:

```text
etas_utils -> etas_syntax
etas_utils -> etas_hir
etas_utils -> etas_types
etas_utils -> etas_effects
etas_utils -> etas_air
etas_utils -> etas_analysis
etas_utils -> etas_runtime
etas_utils -> etas_cli
etas_utils -> etas_lsp
```

`etas_utils` may depend on `etas_core` for shared ids, arenas, or diagnostics
if the algorithms need them, but it should otherwise stay light.

## 10. Implementation Scope

The first `etas_utils` implementation should include:

- lattice traits;
- worklist;
- fixpoint result and iteration limit model;
- a simple fixpoint engine;
- graph traversal traits;
- topological sorting;
- strongly connected components;
- responsibility chain pattern;
- full pass manager traits and deterministic pipeline runner;
- artifact preservation and invalidation primitives;
- analysis cache boundary;
- instrumentation hooks and pass timing/stat containers.

Deferred:

- dominator trees;
- post-dominator trees;
- advanced worklist scheduling;
- widening and narrowing policies;
- incremental fixpoint;
- parallel graph algorithms.
