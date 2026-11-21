<!-- Historical recovery snapshot, archived 2026-11-21. Not a current specification, implementation status, or engineering task assignment. The current language SPEC and main implementation take precedence. -->

# Residual Checks And Runtime Trace Monitors

Status: Architecture target; not an implementation acceptance report.
Owner: Architect
Updated: 2026-11-03

## 1. Contract And Scope

Read SPEC [14, sections 5.1-5.7](../../../etas/docs/design/14-safety-analysis-static-dynamic.md)
and [18, sections 5.2 and 6.2](../../../etas/docs/design/18-reflection-and-checked-code-generation.md).
Residual checks discharge precisely identified dynamic obligations, not missing
type facts, definite violations, arbitrary unknown behavior, or incomplete
replacement compatibility. No new source keyword, effect or public loader is
introduced. This mechanism applies to ordinary programs and compiled A/F values.

There are three different artifacts:

```text
ResidualObligation       # what cannot be proved, with source and contract identity
ResidualMonitorPlan      # immutable, validated executable checking program
ResidualInstallationEvidence # engine-owned binding/coverage evidence for the plan
```

CheckReport.runtime_checks is a descriptive projection of obligations. A string,
report, artifact checksum or configured policy client is not installation proof.
Existing budget, sandbox, output-schema and approval mechanisms remain owners
of their enforcement; the plan can bind to them through verified guard providers.
Do not implement another budget ledger, filesystem guard or approval authority.

The current static TraceSpec automata and Host's partial TraceSpecRuntimeClient
are reusable inputs, not evidence that the complete runtime chain exists.
ResidualCheckSet.checks: BTreeSet<String> must be replaced, not parsed as a DSL.

## 2. Ownership And Layering

Use existing crates, with one shared neutral protocol:

```text
etas_utils::automaton
  generic symbolic/product automata, Matcher and state transitions
etas_code::residual
  stable obligations, predicates, event/plan schemas, pure monitor construction
  and stepping contracts; no HIR, InterpValue, HostValue or engine scheduler
etas_effects::residual and trace_spec
  source/type/effect binding, abstract proof, residualization and neutral lowering
etas_frontend
  project/backend validation, package projection, candidate emission gates
etas_interpreter
  checked-site binding, per-invocation monitor state, execution and restoration
etas_host
  current authority, approval grants, exact workspace/network checks and evidence
future AIR backend
  binds the same neutral plans to AIR sites; no frontend-dependent runtime monitor
```

etas_code already depends on etas_core and etas_utils. Keep language-neutral
monitor algorithms there without adding dependencies on frontend, Host or an
execution engine. Effects must not depend on frontend or interpreter.

Target shared layout, extending existing modules rather than empty scaffolding:

```text
etas_code/src/residual/
  mod.rs
  identity/                 # obligation, plan, site and lexical-scope keys
  model/
    obligation.rs           # typed obligation and applicability
    predicate.rs            # closed predicate/evidence operand vocabulary
    event.rs                # semantic events, outcomes and correlation identity
    scope.rs                # lifetime, exit applicability and ownership
  monitor/
    program.rs              # normalized alternatives/components and state schema
    compile.rs              # pure normalized-clause -> automaton construction
    step.rs                 # pure transition/evidence processing, no scheduling
    matcher.rs              # exact labels and typed evidence reader contract
  validation/
    schema.rs               # references, binding types and semantic versions
    limits.rs               # bytes, nodes, states, registers and work bounds
  artifact/
    codec.rs                # versioned data-only DTOs, not callbacks or Debug text
    fingerprint.rs          # canonical contract/plan/predicate identities
    binding.rs              # checked revision -> semantic plan binding, not ID rewriting
```

The generic automaton remains in Utils. The common normalized clause compiler
is the single semantic source for static and concrete execution. Migrate the
private Before/After builders from effects/trace_spec/monitor/compiler.rs;
static adapters supply abstract matching, engine adapters supply exact matching.
Do not maintain two independent interpretations of the same clause.

Frontend and engine responsibility directories follow their existing layers:

```text
etas_effects/src/residual/
  model/                    # dispositions, obligation templates, call substitutions
  binding/                  # checked action/type/resource evidence to stable keys
  lower/                    # normalized models -> shared monitor plans
  validation/               # coverage, residual eligibility and complete bindings
etas_effects/src/pipeline/passes/
  lower_residual_monitor_plans.rs
  verify_residual_plans.rs
etas_frontend/src/passes/execution/
  residual_support.rs       # target capabilities and checked-program gate
etas_interpreter/src/plan/residual/
  validate.rs               # neutral versions/providers and guard support
  bind.rs                   # stable sites/scopes -> checked engine locations
  coverage.rs               # every required obligation/hook is installed
etas_interpreter/src/eval/residual/
  scope.rs                  # lexical activation and explicit-frame lifecycle
  event.rs                  # checked action/handler/completion event adapter
  step.rs                   # shared kernel invocation, no second clause compiler
  exit.rs                   # all normal/non-local terminal paths
  evidence.rs               # bounded typed projections and Host evidence
etas_interpreter/src/control/residual/
  state.rs                  # live state, alternatives, registers, pending outcomes
  permit.rs                 # checked single-dispatch gate tokens
etas_interpreter/src/api/code/publisher/residual.rs
etas_interpreter/src/api/codec/checkpoint/residual.rs
```

Create cohesive files when their responsibilities are implemented, not empty
facades. Ordinary compilation uses checked scope/constraint identities and does
not require a runtime reflection catalog to create or validate obligations.

## 3. Structured Obligations

The following Rust-shaped schema specifies responsibilities, not new source APIs:

```rust
struct ResidualObligation {
    id: ObligationId,
    origin: SourceRevisionLocation,
    contract: BehaviorContractRef,
    spec: SpecInstanceRef,
    scope: SemanticScopeRef,
    activation: ActivationSiteRef,
    kind: ResidualKind,
    bindings: BindingLayoutRef,
}

enum ResidualKind {
    ActionPattern { pattern: PatternRef, predicate: PredicateRef },
    TraceContract { monitor: MonitorTemplateRef },
    ExistingGuard { provider: GuardProviderId, requirement: GuardRequirementRef },
}

enum StaticDisposition {
    Proven(StaticEvidenceRef),
    Violated(DiagnosticRef),
    NeedsConcrete(ResidualObligation),
    Unsupported(ReasonCode),
}
```

Obligation identity includes canonical declaration/spec instantiation, lexical
owner, clause, source revision, generic substitutions and activation semantics.
Do not deduplicate distinct nested scopes or generic instances because their
messages match. Runtime invocation identity is separate from this static key.
BehaviorContractRef names an ordinary checked source constraint or a frozen
replacement contract; it does not force every source flow into Agent reflection.
Interprocedural summaries carry obligation/template references and substitutions,
not copies of complete plans or flattened human-readable names.

### 3.1 Acyclic Source And Obligation Identity

Use an independent `SourceRevisionKey`; `SourceRevisionLocation` must not require
the existing checked-context `RevisionKey`. That key includes contract and frozen
environment fingerprints, which may depend on the effect analysis creating an
obligation. The source identity is available before that analysis:

```rust
struct SourceRevisionLocation {
    source: StableSourceKey,
    revision: SourceRevisionKey,
    range: TextRange,
}
```

```text
SourceRevisionKey = H("etas.code.source-revision.v1",
                      StableSourceKey, H(exact_source_bytes))

ObligationId = H("etas.code.residual-obligation.v1",
                 SourceRevisionKey, CanonicalOwnerKey,
                 ScopeAndClauseSiteKey, SpecInstantiationKey,
                 StaticBindingRecipeKey, ObligationRuleKey)
```

All inputs use versioned canonical encoding. StableSourceKey identifies the
pre-analysis package/project origin plus logical source path, or a virtual/
generated source's stable origin. It must not use the artifact digest currently
being produced for that package. Equal text in distinct source files is not the
same source; multiple parts of one module retain their separate source origins.
Already validated external package identities can be immutable inputs.

Snapshot exact source bytes without hidden newline/Unicode normalization. Ranges
are zero-based end-exclusive byte ranges in that snapshot; line/column values are
derived. Generated bodies/overlays acquire immutable source identity and source
mapping when constructed, before type/effect analysis; their identity cannot
derive from the new checked revision. Reuse existing source/syntax fingerprint
mechanisms only after verifying their inputs satisfy this separation.

Owner, scope and clause keys use canonical declaration identity and structural
source sites with deterministic disambiguation, not local HIR/SourceId/arena
indices, worklist order or diagnostic messages. Do not promise unchanged IDs
across edits: a changed source snapshot changes its SourceRevisionKey. Repeating
the same input in another process/checkout or traversal order gives the same key.

Static binding recipes preserve canonical type/resource arguments or symbolic
binder/substitution provenance. They distinguish relevant spec/generic instances
without hashing intermediate effect-row solutions or EffectSummary values.
Refining an inferred row changes checked semantic contents, not an already
identified obligation. Distinct call/activation sites remain distinguishable;
runtime invocation IDs remain a separate dimension. No temporary-ID allocation
followed by global renumbering is part of this protocol.

For ordinary code, BehaviorContractRef identifies the checked source constraint
by owner/spec/clause identity, not the fingerprint of the contract currently
being inferred. For replacement compilation it may reference the already frozen
input contract. Do not require the candidate's new summary/contract fingerprint
to construct that reference. Final binding establishes the relationship with the
checked revision after analysis.

Source/obligation identities exclude the current checked contract, frozen
environment, final revision, inferred summary, backend/configuration result,
installation evidence and process-local identity. In particular, the current
FrozenDefinitionEnvironment fingerprint is not a pre-analysis source key: its
implementation includes the contract. A SourceRevisionKey/ObligationId alone is
neither a complete analysis-cache key nor proof of checked behavior. Cache reuse
also validates all environment/dependency/compiler/configuration inputs.

### 3.2 Typed Predicates And Evidence

Bindings name checked parameter/field/result positions or evidence registers.
Static type/spec predicates are discharged from checked TypeFacts; this design
does not move runtime variables into effect/action type arguments. Concrete
payload operands remain separate from static action selectors.

Use a closed typed predicate graph: equality of compatible scalar/identity
operands, checked pattern membership, bounded conjunction/disjunction/negation,
and registered evidence tests. Approval evidence includes matching action/input
identity, actor, accepted outcome, scope and validity. Workspace membership binds
to the existing handle-relative guard, not a textual path-prefix comparison.
Unsupported predicates or missing operands fail closed; no arbitrary Etas/Rust
callbacks, eval(string), effectful getters, or string parsing during enforcement.

Sensitive values are read through bounded borrowed projections. Logs/snapshots
retain redacted or keyed-digest evidence where appropriate, not raw secrets.
A digest cannot replace plaintext for a predicate that needs exact content.
Every guard declares its required evidence, retention and restore policy.
An Approval-shaped result from source code is not a trusted Host grant. Evidence
validation preserves origin and authority; handler resume cannot mint approval
identity, freshness or resource scope merely by constructing a matching record.

### 3.3 Run-Scoped Approval And Explicit Reuse

The default is run-scoped approval in memory. A root run owns its live approval
registry; success, failure or cancellation terminates that authority lifetime.
An approval can have a narrower scope/shorter validity, but cannot outlive its
run by default. Child scopes inherit only applicable narrowing permissions.
Late approval responses after termination are historical outcomes, not grants
for another run. Do not key validity by a reusable request counter alone.

Checkpoint resume starts a new run, including within the same OS process. It
uses current authority and does not inherit the old run's approvals. Taking a
checkpoint while the original run continues, awaiting Host I/O, scheduler yield
or a source handler's continuation `resume` is not checkpoint resume and does
not itself terminate that run. Do not conflate a flow call with a root run.

Keep three concepts separate:

| State | Default checkpoint/resume treatment |
| --- | --- |
| Historical approval request/decision and completed external operation | Preserve as bounded historical/audit facts, not current authority |
| Live approval grants, evidence handles and dispatch permits | Run-local; never deserialize as valid authority or inherit from old run options |
| Monitor progress and evidence-dependent readiness | Preserve trace/branch/rejection progress; invalidate old approval-dependent readiness |

A saved `Approved`/armed flag or previously satisfied approval predicate cannot
authorize future actions after restore. Rebind its evidence slot to unavailable
in the new run and require a current matching approval before it can permit an
action. Do not merely clear AuthorityContext.approvals while retaining that flag.
Equally, do not reset the whole automaton, reopen rejected OR alternatives,
discard prior violations, change the event cursor or erase completed actions.
Restore must explicitly validate this evidence-lifetime transition; if a plan
cannot represent it safely, reject that restore rather than silently resetting.

Record approval-lifetime dependencies even when source analysis proves temporal
order statically. A proof of `G; checkpoint; A` from the original entry does not
prove current approval at A when a new run starts from that checkpoint. Restore
coverage must retain an evidence-aware gate or reject that continuation; a
`Proven` disposition/empty residual set cannot erase this run precondition.
Static invariants and durable completion facts remain distinct from live grants.
This baseline restore gate alone does not turn every CheckReport into guarded.

An old approval result can remain program data or an audit outcome. Replaying
it, replaying the approval boundary ledger, or reconstructing the historical
trace must not repopulate live evidence. No monitor, handler continuation or
grant copied from a prior run becomes trusted through its record shape.

When later work requires approval, use an explicit application/source/handler
reapproval protocol at a checked safe continuation, under the new run's current
authority, limits and normal action tracing. Missing evidence alone must not
implicitly issue Approval.request. Without such a safe continuation/protocol,
reject resume or block the guarded operation with a precise diagnostic before
the side effect. Never rewind arbitrary source steps, repeat confirmed external
work, fabricate acceptance, or automatically retry to reacquire permission.

Cross-run reuse is an explicit application opt-in, separate from enabling
checkpoint storage, trace output or source restoration. The application supplies
a trusted approval-reuse provider and policy; configuration/receipt references
in an untrusted checkpoint cannot enable it or select an arbitrary endpoint.
For every imported approval the provider verifies origin, accepted decision,
principal/authorization domain, expected action/resource/input binding, allowed
scope mapping, validity and applicable revocation policy. Rebind only verified
evidence to the current run, within current authority; do not blindly restore
old grants or extend expiry. Missing/unsupported/unavailable verification fails
closed, never falls back to an old Approved state.

No approval database, signing service, durable receipt store or generic trusted
checkpoint-evidence store is required for default run/checkpoint/resume. Host
provides run-local binding and an explicitly injected reuse-provider boundary;
the application chooses and owns any durable approval backend/retention policy.
Ordinary Memory/Session commit receipts prove storage results, not approval.
Historical audit retention does not keep approval authority alive. This policy
changes no source keyword, effect row or std Approval.request signature and
does not make the rest of a checkpoint trusted.

## 4. Plans And Coverage

A neutral ResidualMonitorPlan contains schema/semantic versions, a semantic plan
hash, source/constraint and known dependency identities, normalized components
and complete OR alternatives, typed predicate/binding tables, semantic site/scope
templates, supported exit checks, limits and exact covered ObligationIds.
References use stable code identities, never serialized HirExprId/AIR offsets.
It does not contain a back-reference to the current final RevisionKey, contract
fingerprint or installation evidence. Their relationship is represented by the
separate checked binding below, not by self-referential hashing.

### 4.1 Semantic Hashes And Final Binding

The dependency direction is fixed:

```text
immutable source -> SourceRevisionKey
  -> obligation identities and semantic monitor plans
  -> converged summaries / validated contract fingerprint
  -> frozen environment and final RevisionKey
  -> BoundResidualPlan / engine installation evidence
```

Contract/summary fingerprints can include stable obligation/semantic-plan
references, but never the final binding that points back to those contracts.
The semantic PlanHash covers normalized plan contents and their versions, not
the current final revision/contract wrapper. A separately fingerprinted
BoundResidualPlan records PlanHash, final RevisionKey, applicable contract and
checked environment/configuration identities. The final RevisionKey must not
hash that bound wrapper, executable envelope or installation evidence.

Once normal contract checking/finalization succeeds, the ordinary output pass or
EmitCheckedCandidatePass builds and validates this outer binding. Validate the
source/owner/site relationship, exact coverage and contract/environment match;
do not regenerate obligation IDs or rewrite semantic plan hashes. A required but
unbound plan cannot be published as executable. Candidate compilation with an
existing frozen contract follows the same direction; no second project check,
placeholder contract or original-summary substitution is necessary.

Engine preparation consumes these checked bindings and creates native site/
provider installation evidence. Installation proof is a later gate, not an
input to effect analysis or contract hashing. Persist both semantic identity
and its final binding; verify their relationship at metadata, cache and restore
boundaries rather than inferring one from a matching display name.

### 4.2 Coverage And Composition

Each required obligation has one explicit discharge path: a monitor component or
an existing guard binding with verified coverage. One component can cover several
obligations, but the mapping must be explicit. Missing, extra-conflicting or
unbound obligations reject; a boolean installed=true is not evidence. Provider
availability alone does not prove the predicate, sites and phases are covered.

Use SymbolicAutomaton/ProductAutomaton and the common clause normalization:

- `+A` / `-A`: exact applicable action-pattern authority/deny checks;
- `G >> A`: local operand permissions plus scope- and outcome-matched G evidence;
- `A << B`: equivalent to `B >> A`, with both local operand permissions and
  matching prior `B` evidence for each `A`; it does not require `A` to occur;
- `S & T`: all components must hold;
- `S | T`: preserve complete alternatives and their states; never borrow an allow
  from S and a temporal discharge from T;
- nested scopes: independently narrowing conjunctions, never one global union.

Keep all still-possible OR branches until evidence removes them. Exact runtime
state does not use static widening or drop obligations. Bound state/register/
pending-obligation growth; exceeding a ceiling stops execution rather than
choosing a convenient state. Generic automaton rejection/acceptance semantics
must be preserved by product construction, including rejecting intermediate steps.

Prior-evidence checks, including both `G >> A` and `A << G`, cannot treat an
attempted, denied, pending or failed approval as accepted evidence. They retain
distinct correlation keys and validated outcomes; do not replace them with one
global approved flag or invent one-shot approval consumption absent from SPEC.
`<<` does not create an eventual-action obligation at scope exit.

### 4.3 Native Kernel And Engine Ports

Expose small typed operations, rather than a universal `check(String)` callback.
The following are Rust API responsibilities, not literal source functions:

```rust
// etas_code::residual: deterministic operations over validated data.
fn compile_plan(model: &NormalizedTraceContract, limits: &PlanLimits)
    -> Result<ValidatedMonitorPlan, PlanError>;
fn initial_state(plan: &ValidatedMonitorPlan, bindings: &ScopeBindings, limits: &StateLimits)
    -> Result<MonitorState, StateError>;
fn preview(plan: &ValidatedMonitorPlan, state: &MonitorState,
    event: &SemanticEvent, evidence: &dyn ResidualEvidenceView)
    -> Result<MonitorDecision, MonitorError>;
fn commit(state: &mut MonitorState, delta: MonitorDelta,
    expected: StateVersion) -> Result<(), StaleTransition>;
fn finish_scope(plan: &ValidatedMonitorPlan, state: &MonitorState,
    exit: ScopeExit) -> ScopeCheckResult;

enum MonitorDecision {
    Permit(MonitorDelta),
    NeedsEvidence(EvidenceRequirements),
    Reject(ResidualViolation),
}
```

ResidualEvidenceView supplies bounded typed borrowed values/outcomes by validated
binding/register references. It does not perform I/O. NeedsEvidence is not Permit:
an engine may wait only through a registered explicit mediation protocol, under
current cancellation/limits, and must re-evaluate afterward. Missing/unobtainable
evidence fails; the kernel never invents an approval request or calls user code.
Commit checks the expected scope-state version; it does not dispatch Host work.

Engine preparation consumes checked plans plus its site/guard adapters and
returns PreparedResidualBindings with private validated coverage. The engine's
publication boundary creates ResidualInstallationEvidence from that result, not
from report data. Execution separately activates state, previews/commits events
and validates scope exits. Async dispatch, locking, cleanup and diagnostics stay
outside the pure kernel. No monitor kernel method owns HIR IDs or an executor.

For the SPEC's `Approval.request >> CompanyEmail.send<WorkAccount>` example,
static analysis can prove the call order yet leave draft equality/freshness as a
typed obligation. A successful approval for draft x populates a trusted evidence
register. Sending draft y previews the same normalized monitor and rejects before
dispatch when x differs from y; a denied/pending approval never populates that
register. No string label, matching escaping row or installed PolicyClient can
discharge this fact. For `Audit.log << StripePayment.charge`, each audit action
instead requires matching prior charge evidence; the expression does not require
an audit action to occur.

## 5. Compiler Pass Protocol

Produce plans through the existing PassManager and abstract-analysis framework:

| Stage | Required inputs | Produced artifact |
| --- | --- | --- |
| AnalyzeTraceSpecMonitorsPass, extended | normalized TraceSpec models, checked summaries, type/evidence facts | per-clause StaticDisposition, structured obligation templates |
| LowerResidualMonitorPlansPass | structured obligations, validated models/bindings, residual policy | immutable neutral plans and coverage map |
| ValidateTraceSpecPass, extended | proof dispositions, lowered plans, existing diagnostics | definite-violation/strict-mode diagnostics and validated trace facts |
| VerifyResidualPlansPass | checked facts, plan graph, all outstanding obligations | VERIFIED_RESIDUAL_PLANS |
| ValidateResidualBackendSupportPass, frontend | verified plans, target capabilities/config | RESIDUAL_BACKEND_SUPPORT and diagnostic projection |
| BuildCheckedProjectPass | normal artifacts plus both residual artifacts | checked program with versioned residual bundle |

Validate ordinary effect contracts as before. An unprovable concrete comparison
becomes NeedsConcrete only if its precise runtime predicate and observation
points can be generated. Missing metadata, lost temporal facts, invalid types,
definite deny violations and arbitrary proof timeouts do not automatically qualify.
ValidateTraceSpecPass must stop treating every monitor Unknown as the same case.

The frontend supplies neutral ResidualTargetCapabilities: predicate/monitor
semantic versions, supported event phases, guard providers, exit/suspension/
restore coverage and resource limits. Effects never queries an interpreter.
If support is absent, reject the required obligation; do not emit a runnable
checked program on the assumption that some Host client will handle it later.

These capabilities describe implemented backend/guard semantics, not live
credentials, configured adapter endpoints, current grants or approved actions.
Ordinary `etas check` can validate supported plans without opening services or
requiring permission to execute them. Service readiness and actual authority are
separate execution gates. Analysis-only consumers without a selected target may
retain plans and source diagnostics but cannot claim executable readiness.

Map SPEC strict/normal/audit modes to existing compilation/deployment options,
not source keywords. Strict requires proof; normal/audit require actual exact
enforcement for residual acceptance. Audit is not permission to skip a known
violation or run without guards. High-impact defaults follow SPEC/deployment
policy; the architecture does not silently relax them to normal mode.

## 6. Reflection And Installation

S_new is inferred from the candidate plus its installed enforcement semantics.
Frozen replacement validation still requires callable/type/spec and complete
caller-visible effect/trace/determinism compatibility. A residual can discharge
only its identified dynamic fact. It cannot repair an unconstrained action set,
unknown callable origin or missing frozen contract by returning guarded.

CheckReport.status has these exact meanings:

| Result | Required condition |
| --- | --- |
| accepted | all obligations statically discharged; no new residual requirements |
| guarded | all remaining obligations have validated plans and supported target bindings; no known/unproved unguardable contract violation |
| rejected | source violation, unguardable obligation or missing required evidence/support |

Baseline runtime checks such as ordinary numeric/schema/budget safety do not
automatically make every report guarded. This classification concerns the
outstanding proof obligations identified for the checked candidate/contract.

check does not start an invocation, request approval, acquire a connection or
mutate monitor state. It can produce guarded by proving plan validity and
installability for the selected target. Compiler-service operational failure
remains CompileError(service), not a rejected source report.

compile consumes fingerprint-matching checked plans and prepares them through
the normal CandidatePublisher. Preparation returns engine-owned
ResidualInstallationEvidence, bound to the candidate revision, contract, plan
hashes, complete obligation coverage, checked site/ABI mapping and backend/guard
semantic versions. It binds factories and execution hooks; it does not execute
the candidate or activate one global monitor shared by all calls.

The callable may be returned only after native validation of that evidence.
A descriptive CheckReport or serialized receipt cannot manufacture it. Backend
changes between check and compile cause revalidation/rejection. The same gate
applies to custom compiler-handler results, ordinary preparation and resume.
Actual calls allocate independent scope instances and execute the pinned plans.

## 7. Runtime Events And Mediation

Extend the existing typed action mediation, not only Host I/O wrappers. A
language action can be interpreted entirely by a source handler and must remain
visible. Std forwarders and agent inference also use their checked action origin.

The semantic input is a typed stream of action attempts, governed dispatch,
handler/default completion or failure, accepted/denied approval evidence and
scope enter/exit/suspend events. Each event binds canonical action identity,
static selectors, payload projections, source revision, invocation/scope,
semantic attempt identity, phase and outcome. Host request IDs are not semantic
action IDs: one action may use several network chunks or retries.

An attempted-action audit event records intent; it is not successful execution
or valid guard evidence. Checked normalization identifies the governed transition
and its evidence phase. Do not advance all temporal components once at attempt
and again at default execution. Static abstract events and runtime events must
use the same normalized phase/outcome semantics.

For a governed action, mediate as follows:

1. Bind evaluated arguments through CheckedActionInvocation and record the
   attempt, including attempts later rejected. Validate required request-level
   constraints and determine scoped-handler/default dispatch without executing it.
2. Preview the relevant monitor transition and concrete predicates. A denied or
   failed guard blocks dispatch. Explicit approval mediation may suspend before
   committing the target transition; it does not count as accepted approval.
3. Before any governed external side effect, revalidate current authority,
   evidence/freshness, limits and exact sandbox/resource constraints. A handler
   forwarding to default cannot skip this gate or gain authority from a resume.
4. Commit the permitted semantic transition under the scope owner's ordering
   protocol, then dispatch once. On actual completion/failure feed the correlated
   outcome; only validated matching outcomes can discharge prerequisite evidence.
5. Preserve downstream requests from handler bodies as their own checked actions.
   A dry-run handled write is not a real filesystem-write completion, and one
   resumed semantic request is not counted again at every continuation hop.

Only events applicable to the normalized clause drive its transition. Handler
dispatch, handled requests, default executions and failures are distinct phases;
the runtime may not choose the weakest phase to authorize a request. If a source
form lacks a representable exact phase/evidence rule, reject that residual form
instead of inventing an implicit handler or approval policy.

Guard permits are internal, single-dispatch objects bound to scope/state version,
plan/contract, action attempt, payload identity, authority and evidence validity.
Do not hold a monitor mutex across Host awaits. If authority, payload or monitor
state changes while waiting, discard/recheck a stale permit. Serialize shared
scope decisions before irreversible dispatch; state in separate calls is separate.
Permission checks do not replace handle-relative OS guards at the I/O boundary.

The engine's checked semantic event sequence and bounded monitor state are
authoritative; redacted exporter output is not the input to enforcement. This
does not require retaining complete history in RAM: keep indexed state/registers,
correlation and a cursor, with existing durable trace storage when configured.
Disabling tracing/export or sampling telemetry cannot disable monitors. Do not
reconstruct evidence from pretty-printed log strings.

## 8. Scope, Failure And Concurrency

Own monitor state in the evaluator's explicit scope/frame machine, not Rust
recursive call frames or a global map keyed only by spec name. Calls inherit
active outer scopes and activate their own checked scopes. Cross-revision calls
retain outer restrictions while using the callee's pinned plans and site mapping.

Normal return/tail completion validates scope acceptance before releasing the
result. Non-local return, Error propagation, handler finish,
abort/never termination and cancellation all visit the scope-unwind protocol.
Suspending for Host I/O, yielding or checkpointing is not scope exit. Error
capture by ? does not erase a violation. `<<` does not create a pending
after-action obligation.

An interrupted/failed scope is recorded incomplete or violated as applicable,
never satisfied. Preserve cancellation/primary failure as the terminal cause
with related obligation diagnostics; do not auto-run audit, compensation, retry
or approval code to fabricate evidence. Stronger guarantees requiring durable
compensation belong to explicit application mechanisms.

Concurrent children inherit narrowing constraints. A shared trace-spec scope
uses one serialized event owner and explicit correlation/causality, not timestamps
or local branch order as proof. Child completion/cancellation and all pending
external outcomes must settle before final scope acceptance. Do not clear state
when one child exits, leak approval across tenants or release siblings on a
monitor failure. Stop new forbidden work and use existing cancellation/cleanup
ownership for outstanding operations; unknown side effects stay unknown.

## 9. Checkpoint And Replay

Extend the engine checkpoint schema together with monitor installation support.
Persist plan/semantic versions, code/contract/generic binding identities,
historical scope/branch/component progress, bounded historical evidence and
binding references, in-flight outcomes, state versions and semantic event cursors.
Run-local approvals and dispatch permits are not portable authority. Capture
the cursor/progress atomically with machine/operation state; no checkpoint may
record a completed action with its monitor step lost.

Restore validates plans/site ownership and bounded typed state, then applies
the [run-scoped approval policy](#33-run-scoped-approval-and-explicit-reuse).
Use the new run's authority and remaining budgets. Invalidate old live approvals
and every approval-dependent Approved/armed readiness; preserve historical
progress, denied branches and confirmed external completion. This is an explicit
restore transition, not default-to-empty monitor recovery. Missing/malformed
state or an unrepresentable transition rejects. Only an explicitly enabled
application provider may revalidate persisted approval references for reuse.
Recheck resource identity and applicable time budgets; a new TraceId retains
lineage, not authorization, without losing the historical semantic cursor.

Replay preserves attempt/completion identity and historical facts exactly once.
Its historical approval events never create current grants or mark current
approval predicates satisfied. Live events use the same transition semantics
plus current evidence validity; historical replay is not fresh approval.
Restoring an uncertain outcome does not assume success or redispatch blindly.
Retry/resampling cannot refresh approval or budget by itself. Ordinary resume
needs no approval database; inability to safely proceed without new evidence
produces a precise stop/rejection, not automatic reapproval or repeated I/O.

## 10. Security, Bounds And Acceptance

Validate aggregate encoded/decompressed size, graph/node/state counts, binding
types, predicate depth, registers, correlation-key bytes, pending count, active
scope depth and transition work before allocation or execution. Plans are data,
not executable plugins; canonical fingerprints include semantic versions.
Secret evidence is not exposed in Debug/diagnostics/report/trace/checkpoint.
Persisted keyed digests require an authorized key lifecycle or explicit
unrestorable-evidence error; never compare a redacted placeholder as real data.

Index plans by canonical action/phase/scope to avoid rescanning all history per
event. Share immutable plans across calls/revisions; budget each live state.
Do not weaken checking when limits, cancellation or evidence projection fails.
Reuse existing typed runtime failure/authority/limit categories and real source
locations; do not disguise a runtime violation as compile rejection or approval.

Required acceptance includes:
- identity determinism across processes/checkout relocation, file/part
  disambiguation, changed-source separation, stable IDs during SCC iteration and
  preservation through final binding; source/semantic hashes have no dependency
  on current final contract/revision/envelope/installation hashes;
- each clause/combination: allow/deny, implicit >> permissions, complete OR
  alternatives, nested narrowing and source/static/runtime agreement;
- dynamic values, nominal/resource identity, same/different approved payload,
  denied/failed/stale approval and prior-evidence checks across actual scope exits;
- user/scoped/default/forwarded handlers, std wrappers, agent/model-selected
  tools, source/cross-package/compiled callables and exception/non-local exits;
- delayed Host completion, stale permit races, concurrent siblings, cancellation,
  checkpoint/replay and no double counting across suspension/default forwarding;
- approval lifetime: default operation without a database; same-process new-run
  and cross-process resume cannot inherit grants, old Approved states, ledger
  results or audit evidence; explicit safe reapproval does not repeat completed
  work; taking a checkpoint/handler continuation in the live run does not expire
  otherwise valid approval;
- optional reuse: disabled by default, application-enabled verified reuse only;
  forged/expired/revoked/mismatched/unavailable-provider evidence never permits;
  historical progress and rejected OR branches survive evidence invalidation;
- check has no runtime effects, compile installs all required guards, installation
  failure returns no callable, and forged reports/custom handler results reject;
- ordinary execution and reflected A/F use the same guard path; malformed
  metadata/unknown provider/schema/missing coverage fail closed;
- deterministic differential/property tests for static proof vs concrete traces,
  bounded fuzzing, redaction, allocation/work ceilings and instrumentation-off runs.

Until these are implemented, keep explicit unsupported-residual rejection. That
is a safety boundary, not completion. No empty handler, string token, no-op monitor
or unsafe TraceSpecRuntimeClient mode can satisfy this architecture.
