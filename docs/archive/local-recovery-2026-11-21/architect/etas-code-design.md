<!-- Historical recovery snapshot, archived 2026-11-21. Not a current specification, implementation status, or engineering task assignment. The current language SPEC and main implementation take precedence. -->

# Checked Code Contracts And Compiler Service

Status: Architecture target aligned with SPEC chapter 18; not implementation acceptance.
Owner: Architect
Updated: 2026-10-31

## 1. Ownership

The language reference is
[Reflection and Checked Code Generation](../../../etas/docs/design/18-reflection-and-checked-code-generation.md).
Internal Rust names below are architecture names, not additional source keywords
or standard APIs. Chapter 18 defines both AgentDef<A> and ordinary-flow
CallableDef<F> editing/compilation; bare CallableDef is read-only type erasure.

Keep one cohesive shared-contract crate, `etas_code`, in `etas-core`. It owns
definition identities, immutable descriptor graphs, draft references, contract
references, diagnostics, artifact envelopes and neutral residual monitor plans.
It does not own parsing, AST/HIR,
type/effect analysis, evaluator state, package acquisition or an execution engine.

This is a substantial shared code-definition protocol, so do not grow the
source/span utility crate into a compiler-service implementation. No separate
reflection, revision, descriptor and dynamic-compile crates are needed.

Dependency direction:

```text
etas_code -> etas_core + etas_utils            # shared contracts and pure automata
etas_std -> etas_code                          # declarations describe those contracts
etas_host::compiler -> etas_code               # effectful compiler-service port
etas_frontend::code -> etas_code               # parser/checker implementation
etas_driver::compiler -> frontend + host       # native service composition
etas_interpreter -> frontend + host + code     # checked-HIR publisher/execution
future optimizing/runtime -> code             # backend-specific publication
```

Core never depends on the frontend or interpreter. Standard declarations do not
execute code. The compiler-neutral protocol never carries HirItemId, TypeId,
CheckedProject, InterpValue, a live Frame, or a mutable FrontendSession.

## 2. Layered Package Structure

Target files and responsibilities:

```text
crates/etas_code/
  Cargo.toml
  src/
    lib.rs
    identity/
      mod.rs
      definition.rs       # package/module/declaration + kind + impl/spec identity
      source_revision.rs  # pre-analysis logical-source identity and exact-byte revision
      revision.rs         # code/environment/contract/config fingerprints and parent
    descriptor/
      mod.rs
      callable.rs         # SignatureInfo and parameter/generic binder descriptions
      types.rs            # interned descriptive TypeInfo graph, not type evidence
      definitions/        # primary agent and typed/erased callable headers
      annotation.rs       # static annotation values, stable declaration references
      behavior.rs         # escaping/requested/default-handled rows and trace refs
      dependency.rs       # binding/environment/locked dependency descriptors
    syntax/
      mod.rs
      reference.rs        # immutable structured-syntax object reference + source key
      operations.rs       # synchronous in-memory syntax/descriptor operation ports
    definition/
      mod.rs
      snapshot.rs         # frozen base snapshot and structural draft edits
      analysis.rs         # Checked / BaselineOnly / Unavailable internal state
      lease.rs            # retention ownership of code/syntax objects, no OS handle
    contract/
      mod.rs
      frozen.rs           # opaque authoritative contract + compatibility rule version
      inclusion.rs        # neutral behavior/trace inclusion; no frontend checker
    diagnostic/
      mod.rs
      report.rs           # acceptance/rejection/residual obligations and fingerprint
      detail.rs           # typed expected/actual, related locations, analysis paths
    residual/
      identity/           # obligation/plan/site/scope identity
      model/              # typed predicates, events, bindings and scope contracts
      monitor/            # normalized plan builder/stepper using Utils automata
      validation/         # schema/reference/coverage/resource limits
      artifact/           # bounded data-only codec and canonical fingerprints
    artifact/
      mod.rs
      envelope.rs         # schema/compiler/backend/dependencies/contract/revision
      limits.rs           # bounded node/depth/bytes validation
      reference.rs        # checked code object references, never execution authority
```

Use immutable shared backing for graphs and interned descriptor IDs. Recursive
TypeInfo graphs need explicit references and cycle handling; they must not copy
the entire compiler type universe into every descriptor. Fingerprints use
canonical structured encodings, not Debug output or process-local IDs.

Residual contracts and their complete lowering/installation protocol are in
[Residual Checks And Runtime Trace Monitors](etas-residual-checks-design.md).
The shared pure monitor kernel compiles already-normalized clauses; it does not
resolve source specs or evaluate HIR/AIR. Static analysis and concrete engines
provide different evidence adapters over that same semantics. Ordinary source
constraints use checked scope/contract identities without a reflection catalog.
The crate owns no invocation state, live evidence store or engine scheduler.

Keep `ResidualObligation`, `ResidualMonitorPlan` and engine-owned
`ResidualInstallationEvidence` distinct. `RuntimeCheckInfo` remains a descriptive
report projection, not a callback, runtime DSL or installation receipt. Data-only
plans use canonical IDs and explicit semantic versions; serialized closures,
local HIR IDs and parsing check-description strings are forbidden.

## 3. Identity And Value Invariants

`DefinitionKey` identifies the original nominal agent or callable declaration
using canonical package/module/declaration identity. Method identity also includes
its owning implementation/spec and selected method. Display names alone are
not identities; two methods with the same spelling may differ.

`RevisionKey` identifies one implementation and its frozen environment/contract/
compiler configuration. It does not allocate a new source nominal type A.
`RevisionInfo` includes content fingerprint and available parent information.

### Source Identity Before Analysis

Use independent `StableSourceKey` / `SourceRevisionKey` types for immutable source
locations. SourceRevisionKey hashes the logical source origin and exact bytes,
not a contract, checked environment or final RevisionKey. SourceRevisionLocation
contains that source key/revision and a byte range. This is separate from both
the post-contract checked-context revision and a live execution invocation.

The normative [acyclic identity protocol](etas-residual-checks-design.md#31-acyclic-source-and-obligation-identity)
defines source/site/generic keys and exclusions. Obligation identities use these
pre-analysis coordinates and stable binding recipes; SCC refinement never
renumbers them. Ordinary source constraints are referenced by owner/spec/clause,
while a replacement may reference its already frozen input contract.

Semantic obligation/monitor hashes precede current contract/environment/final
revision hashes. Check finalization creates a separate BoundResidualPlan linking
those semantic hashes to the final revision and contract. It does not patch
obligation IDs or hash the final wrapper back into its own contract/revision.
The [final-binding protocol](etas-residual-checks-design.md#41-semantic-hashes-and-final-binding)
also applies to cache, metadata and checkpoint validation. Keep existing
RevisionKey semantics; do not rename it to SourceRevisionKey or populate its
contract with zero, an old summary or a temporary token to unblock analysis.

Source identity is not analysis validity: compilation/configuration/environment
changes can preserve a source/obligation key while invalidating checked artifacts
and requiring a new final binding. Do not use these small keys alone as caches.

### Checked Values And Revisions

`AgentDef<A>` and `CallableDef<F>` use immutable definition snapshot references
plus checked target-family/interface evidence supplied by the compiler/engine.
F is an ordinary callable type including generic/escaping-row bindings, not a
new nominal type. Core transports stable contract references, never frontend
TypeId or a type reconstructed from runtime SignatureInfo.

Typed callable descriptors may project to the dedicated erased CallableDef view
for inspection, including Array<CallableDef> and agent method lists. Erasure
shares immutable metadata/revision/dependency backing but supplies no edit or
compile port. It has no inverse conversion and is not a general omitted-generic
rule. Direct agent-method reflection is erased; no detached method compiler or
method installer is introduced.

An edit preserves its family, authoritative base identity, static A/F signature,
contract and permitted environment. It never changes a live agent or flow. The
implementation must not rely on editable descriptor fields as its source of
authority or contract evidence.

Keep the authoritative frozen caller contract separate from a revision's actual
checked behavior summary. A callable reference identifies both its contract and
the specific checked revision; it must not label the old body's summary as the
new body's result. An inferred contract may legitimately share immutable backing
with the baseline summary that established it. Compatibility requires the new
summary to satisfy that unchanged contract, not fieldwise old/new equality.
Successive edits retain the same contract even when the latest actual summary
is narrower. These are neutral identity/provenance contracts, not a new public
source type or effect; compiler-local symbolic origin recipes stay in frontend.

An edit receives a fresh draft/content fingerprint. The base revision is kept
as parent/reference metadata, not reused as the edited content's fingerprint.
No new public draft type is implied by an internal DraftKey.

`SyntaxBlockRef` points to a retained structured syntax object. The backend
syntax store owns the native AST and source data; the shared crate owns
references/lifetimes and neutral descriptors, not another Etas AST. A string
plus a marker claiming "parsed" is not a SyntaxBlock implementation. No unchecked
Any/downcast or arbitrary source-created numeric ID may recover a code object.

The compiler provides the syntax/definition object store and a synchronous
`CodeSyntaxOperations` port. Parsing, formatting, with_body and diff only inspect
or allocate in-memory immutable code objects. The port must not access disk,
network, user locals, or an external compiler process. Allocation limits may
cause a typed operational failure under the eventual support-error contract.

Plain TypeInfo/SpecInfo/EffectRowInfo values remain descriptions. Copying or
constructing them cannot establish type equality, spec evidence, trust,
permissions or a checked executable.

## 4. Analysis Invalidation

Internal analysis state is explicit:

```rust
enum DefinitionAnalysisState {
    Checked { input: AnalysisFingerprint, summaries: SummaryRefs },
    BaselineOnly { base: RevisionKey, summaries: SummaryRefs },
    Unavailable { reason: AnalysisUnavailableReason },
}
```

This is an internal model. SPEC requires edited A/F descriptors to preserve
declared_bound and expose escaping/requested/default_handled, determinism and
trace as None until new analysis is obtained. BaselineOnly is internal frozen
contract evidence, not the public current summary. Invalidate dependent auxiliary
summaries too. A check report has new fingerprint-bound analysis without
mutating the draft. Do not substitute empty effects or deterministic behavior.

Requested actions and escaping rows are independent. Descriptor graphs must
retain parameterized action identities, type arguments, row variables and binder
identity; rendering to strings is a presentation step. TraceSummary references
the existing trace analysis representation, not another policy engine.

Compatibility preserves input/output/generic/spec obligations, escaping and
requested-action upper bounds/scopes, caller-visible temporal guarantees and
promised determinism. Neutral trace inclusion utilities may be shared here,
reusing etas_utils automaton/graph/fixpoint infrastructure. Frontend supplies
checked summaries and type judgments. Inclusion preserves permitted entry states,
outcome distinctions and intermediate violations, not merely final states or
unordered action sets; unproved compatibility rejects. F alone is not the full
contract and stored descriptors never constitute proof.

## 5. Host Compiler Port

Add a compiler domain beside network/filesystem/memory in etas_host:

```text
crates/etas_host/src/compiler/
  mod.rs
  protocol/
    mod.rs
    request.rs         # Check and Compile payloads, frozen inputs and request ID
    response.rs        # report or inert checked-artifact reference
    error.rs           # operational errors, distinct from invalid candidate source
  service/
    mod.rs
    client.rs          # CompilerClient / CompilerService abstraction
  validation/
    mod.rs
    identity.rs        # draft/base/contract/dependency/config matches
  limits/
    mod.rs
    accounting.rs      # queued work, allocation, deadline and cancellation accounting
```

Code.check and Code.compile use the ordinary request context: authority, trace,
request ID, cancellation and execution budgets. They join existing operation
ownership, completion and cleanup contracts. No compiler-only cancellation token
or unmanaged spawn_blocking pool is allowed.

Requests distinguish typed agent and ordinary-flow targets and carry stable
definition/interface/contract provenance; raw erased metadata is not a target.
Both check overloads return CheckReport; compile returns the corresponding A/F
through engine publication, never a backend graph in a Host response. Dispatch
uses checked overload/family evidence, not a runtime signature string.

Reports use rejected/accepted/guarded and include diagnostics, revision, locked
dependencies, snapshot fingerprint, available behavior and residual checks.
ParseError contains diagnostics; CompileError has kind rejected/service and
diagnostics. Locations carry source/revision and zero-based end-exclusive byte
ranges. Checking invalid code returns a rejected report; compiler-service failure
raises Error<CompileError>(service), while compile rejection uses kind=rejected.
Authority denial, cancellation and limits remain their own enforcement outcomes.

The protocol returns reports or opaque inert checked-artifact references.
Backend-specific compiled graphs remain in an engine-owned publication store.
Such references do not grant execution authority and cannot bypass artifact
compatibility validation.

Compiler work inherits parent limits and cancellation. Account queue waiting in
WallTime and measure CPU/wall time with explicit duration units, memory in bytes.
Retries/new revisions/cache misses do not reset budgets. CPU/memory ceilings
belong to service/runtime configuration; Tokens/ContextTokens concern actual
model use, not ordinary compilation. Pure code operations also obey execution
limits. Adapter tests must prove bounded allocation/work/cleanup, not merely
that an awaiting Future can be cancelled.

The pure CodeSyntaxOperations port is separate from effectful compiler dispatch.
Do not introduce Code.parse/Code.reflect actions absent from SPEC merely because
a compiler implementation supplies parsing. Nor may a PureBuiltin descriptor
create a dependency from etas_builtin back to frontend.

## 6. Native Adapter And Publication

The native adapter lives in etas_driver::compiler. It composes an in-process
frontend compilation session with a `CandidatePublisher` supplied by the chosen
engine. CandidatePublisher is a frontend-facing port accepting immutable checked
candidate data; the interpreter implements it without making driver depend on
interpreter. The CLI selects and wires these existing services.

```text
checked action / custom scoped handler
  -> normal Code default handler
  -> Host compiler protocol and authorization
  -> driver CompilerService adapter
  -> frontend candidate check/compile pipeline
  -> engine CandidatePublisher (prepare, never evaluate)
  -> inert validated artifact/revision reference
  -> caller receives an ordinary agent A or ordinary flow F
```

The adapter must not invoke a shell command or re-read a mutable source pathname.
Frontend worker results are immutable candidate data. Schedule engine-specific
publication/preparation on the owning engine executor before returning A/F. Do
not send InterpValue, Rc-backed frames or continuations through a blocking worker
or add unsafe Send implementations to make the adapter compile.

Compilation may publish an immutable candidate into the engine's local artifact
store; it must not update a global agent/flow binding or execute the candidate.
Cancellation cannot turn unfinished compilation into an accepted report.
Late worker results are disposed/accounted for through normal operation ownership.

Frontend check can report guarded after validating all residual plans and target
support, without executing or activating the candidate. Compile publication
additionally requires native engine installation evidence for the exact revision,
contract, plan hashes, obligation coverage, checked sites and guard-provider
versions. The publisher binds factories/hooks before returning A/F; each actual
invocation owns its own monitor state. A report, checksum or configured Host
client cannot substitute for that evidence. Custom compiler handlers face the
same gate, and unguardable compatibility failures remain rejected.

The future AIR engine publishes its own executable artifact. The Host protocol
must not force AIR to execute HIR or force Phase 1 to introduce bytecode.
Its frontend-facing CandidatePublisher adapter belongs in optimizing/toolchain
composition; runtime exposes an engine-neutral AIR installation port and must
not acquire a frontend dependency merely to implement that trait.

## 7. Package And Artifact Contracts

The crate-specific producer/wire/consumer contract is recorded in
[Reflection In Package Metadata](etas-package-metadata-reflection-design.md).

Extend etas_package_metadata with a versioned optional code-definition section.
It includes canonical definition/method IDs, descriptor graph references,
analysis/config/dependency fingerprints, annotations and body-availability
information. Retained syntax/body payload is present only under an approved
retention/export policy.

Public signature availability and body availability are independent. Missing or
hidden body data means None, not an empty body or an implicit filesystem load.
Publishing public metadata does not implicitly publish private source bodies.

The package manager transports and verifies the compiler-produced artifact.
Frontend owns its semantic validation; the manager must not synthesize reflection
definitions or trust code summaries as executable evidence. Re-exports preserve
canonical identity and do not upgrade body visibility.

The executable artifact envelope records schema/compiler/backend compatibility,
code/dependency/contract fingerprints, revision lineage and necessary relocation
information. A matching digest proves content identity, not author trust or
permission to execute. Enforce byte/node/depth/graph bounds before allocation.
Plain metadata TypeInfo is never deserialized into a trusted engine type by shape.

There is no public std.compiler.load, Code.load or binary executable export API.
Binary package metadata and derived engine/cache artifacts remain internal
validated formats, not public code-restoration authority. Public restoration
saves body source, canonical target, frozen contract/dependency/config
fingerprints and locked surrounding definitions through ordinary storage. It
resolves a statically known typed target, validates inputs, parses, applies
with_body and compiles again under current authority/limits. Missing data or
changed contracts reject; no original-target fallback, mutable-path substitution,
stored-summary proof or inverse erasure from CallableDef is permitted.

## 8. Memory And Acceptance

Reflection references one immutable retained revision graph; it does not clone
ProjectOutput, the full body fact maps, or one compiler session per reflected
value. with_body shares the base graph and stores one replacement syntax object.
Pin code required by live agent/flow values, invocations and checkpoints; evict
only unreferenced derived data. Account retained bytes without double-counting
shared backing. Revisions cannot be reclaimed while an active invocation uses
them, and a tiny descriptor must not accidentally retain all historical projects.

Tests must cover copy/draft isolation, hidden bodies, canonical IDs, generic row
descriptors, stale environment/config/check reports, forged handles/artifacts,
cyclic/deep descriptor graphs, bounded parsing/compilation, cancellation races,
default/custom handler behavior, current authority on execution, and artifact
schema rejection. Completion also requires source-level tests through frontend,
interpreter and the public std APIs; Rust descriptor construction alone is not
feature acceptance.
