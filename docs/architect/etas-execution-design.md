# Shared Execution Lifecycle Design

Status: `Architecture accepted; implementation acceptance pending`

Owner: `Architect`

Last updated: `2026-09-09`

## 1. Contract And Scope

`etas_host::execution` owns engine-neutral execution scopes, cancellation,
operation ownership, and shutdown observation. The checked-HIR interpreter and
the future AIR runtime reuse this implementation; each retains its evaluator,
scheduler, values, continuations, handler dispatch, and checkpoint codec. No new
crate or common execution IR is required.

The language basis is the current [Concurrency SPEC](../../../etas/docs/design/16-concurrency.md),
especially structured ownership, cooperative cancellation, budget failure,
dynamic handlers, and trace visibility. The external-run cancellation behavior
below is an architecture decision. It does not introduce a source `Cancel`
effect, keyword, or catchable error; any new source-visible API must be reviewed
against the SPEC before adding standard declarations.

Tracked work: [umbrella RFC](https://github.com/etas-project/etas-core/issues/10),
[Host lifecycle](https://github.com/etas-project/etas-core/issues/11),
[Interpreter](https://github.com/etas-project/etas-interpreter/issues/4), and
[CLI](https://github.com/etas-project/etas/issues/3).
[Storage](https://github.com/etas-project/etas-core/issues/12) shares the operation
completion contract through [Bounded Versioned Storage](etas-storage-design.md);
[Workspace](https://github.com/etas-project/etas-core/issues/13)
is specified separately in [Workspace Binding And Isolation](etas-workspace-design.md).
Operations retain authorized directory bindings until actual completion;
cancellation does not retarget/revoke an opened object or roll back committed
filesystem changes. This document does not define an application lease,
takeover, or conversational-state service.

## 2. Ownership And Layout

```text
crates/etas_host/src/execution/
  mod.rs                         # narrow public exports
  cancellation/
    mod.rs
    source.rs                    # CancelSource, restricted CancelSignal
    reason.rs                    # reason and originating scope
  scope/
    mod.rs
    identity.rs                  # invocation-local ScopeId
    state.rs                     # admission and lifecycle transitions
    registry.rs                  # owned children and active registrations
  operation/
    mod.rs
    context.rs                   # scope/signal and trace correlation
    registration.rs              # OperationRegistration, completion ownership
    outcome.rs                   # evidence of external completion
  shutdown/
    mod.rs
    policy.rs                    # bounded cleanup and wait policy
    report.rs                    # immutable terminal report / pending work
    wait.rs                      # observe termination without losing ownership
```

`context/` continues to own authority, trace, request IDs, budget, and host
errors. Cancellation has one implementation under `execution/cancellation/`;
do not create a parallel `context/cancellation/` tree. Service domains own
concrete cancellation mechanisms. `execution/` coordinates lifetime and records
outcomes; it does not dispatch language handlers or schedule HIR/AIR tasks.

Core structures have these contracts; fields remain private so consumers cannot
change state or bypass admission checks:

| Structure | Contract |
|---|---|
| `ExecutionScope` | Parent/child ownership, admission, lifecycle and completion observation |
| `CancelSource` | Authorized stop request; publishes a reason before notifying observers |
| `CancelSignal` | Read-only cancellation observation and waiting; no parent/sibling cancellation authority |
| `OperationContext` | Borrowed/cloneable live operation context with scope, signal and existing trace/request correlation |
| `OperationRegistration` | Registers work before dispatch; retains ownership until actual completion/cleanup |
| `TerminationReport` | Immutable local termination result plus external completion evidence and cleanup failures |
| `StopWait` | `Terminated(report)` or `TimedOut(pending)`; timeout does not change lifecycle to terminated |

No structure contains `CheckedProject`, HIR/AIR IDs, engine values, source
continuations, or frontend effect facts. Engines attach their own source origins
to correlation IDs for diagnostics. Tokens, registries and OS handles are live
state, not serializable metadata.

## 3. Scope Tree And Reuse

```text
RunScope
  -> BranchScope A -> action invocation -> handler -> host operations
  -> BranchScope B -> action invocation -> handler -> host operations
```

Create a scope for a run, a structured concurrent branch/group, or a distinct
local deadline/lifetime boundary. Ordinary flow calls, handler applications and
actions inherit the current scope; they do not each spawn a task or allocate a
new cancellation subtree. Handler scope and execution scope are separate
structures connected by engine-owned execution context.

Use `tokio_util::sync::CancellationToken` for notification and child propagation.
Use `child_token()` for a child scope; `clone()` shares cancellation state and
must not stand in for a child. Tokens remain encapsulated so a service cannot
cancel its caller. Token propagation is not an atomic transaction across a
whole tree; scope admission supplies the required synchronization.

Use Tokio task handles/`TaskTracker` where actual asynchronous tasks are owned.
Do not create an executor or spawn one Rust task per pure handler. A tracker is
not the admission gate, does not collect semantic results, and is not proof of
process or remote-operation termination. A strong parent owns live children;
backreferences must not create reference cycles.

Cancelling a parent propagates downward. Cancelling a child does not itself
cancel a parent or sibling. The execution engine implements SPEC combinator
policy: an unhandled error in `join`/`try_join` cancels unfinished siblings,
`race` cancels unfinished losers after the first successful result rather than
the first error, and `collect` does not cancel siblings merely because one
result is an error. The engine waits for local child settlement before the
concurrent expression returns; core does not select language result values.

## 4. Lifecycle And Concurrency Invariants

```text
Running  -> Draining -> Terminated      # normal body completion, await children
Running  -> Stopping -> Terminated      # cancellation/failure, stop then await
Draining -> Stopping                   # cancellation/failure during drain
```

The engine's body result is separate from scope lifecycle. `Draining` rejects
new unowned/root work but lets already-owned children run and create their own
properly nested work. `Stopping` closes ordinary admission throughout that
scope's subtree. It permits only bounded cleanup of already-owned resources.

- Serialize stop/admission with a short state/registry critical section. A new
  registration checks applicable ancestor gates as well as its local gate. A
  stop and registration race must either register owned work before stop or
  reject it; never start unregistered work after a successful stop transition.
- Registration preceding cancellation is not proof that dispatch already
  happened. The adapter checks the signal before its irreversible boundary;
  once dispatch wins the race, it must report the actual completion evidence.
- Publish the first effective cancellation reason once, before notification.
  Record later causes as evidence without replacing it. The engine determines
  the terminal failure/result according to language rules, independently of
  which request first triggered cancellation.
- Publish each operation terminal result exactly once. Completion/cancellation
  arbitration must not rely on `select!` branch polling order. Release locks
  before waking callbacks, awaiting I/O, waiting for children, or cleaning up.
- Normal completion waits for all owned local work. A stop request arriving
  after terminal publication cannot rewrite that terminal result.
- Dropping an execution owner or an active operation registration must request
  cancellation or leave unfinished work with an explicit completion owner. It
  must not remove unfinished work from accounting or claim that dropping a task
  handle stopped the task. Dropping an observer/control handle or a join waiter
  alone does not cancel the owned execution.

`stop(reason)` is idempotent notification, not a blocking operation. `join()`
observes actual termination; cancelling a join waiter leaves the run owned.
`wait_stopped(deadline)` bounds the caller's wait and returns `StopWait`. On
timeout the scope stays live and observable; repeated waits do not rerun work.
Keep a completion primitive that late waiters can observe without lost wakeups.

The Interpreter's [controlled invocation API](../../../etas-interpreter/docs/architect/phase1-interpreter-design.md#31-controlled-invocation-api)
allocates one fresh scope per run/resume and keeps single-use execution
ownership in `RunInvocation`, outside the shared crate. Ownership starts before
first poll so dropping an unstarted invocation cannot strand a claimed body.
Core owns scope state/registration synchronization; it does not store the
invocation, borrow frontend data, or drive its future. The public `RunControl`
only observes/requests stop and does not expose mutable root-scope access.

`TerminationReport` is engine-neutral and reusable by multiple observers.
`RunOutcome` and `InterpValue` stay in the Interpreter; its final `RunResult`
requires a terminal report, while pending wait results remain `StopWait`.
These are different responsibility boundaries, not duplicate lifecycle state
machines. A failure that caused cancellation is not automatically replaced by
a cancelled language result when the shared report is assembled.

## 5. Effect And Host Boundary

The engine mediates dynamic action invocations, not static effect families:

```text
scope admission -> register action occurrence -> checked authority/handler
  -> zero or more lower actions / registered host requests
  -> publish completion evidence -> release completed registrations
```

Pure handlers can produce no Host request. A handler lowering one action to
several requests produces a one-to-many correlation. Reuse existing request,
trace and boundary occurrence identities; never deduplicate by action name.
Retries have distinct attempt identities. Preserve parent occurrence links
without using interpreter-specific identity types inside Host primitives.

All host-facing request kinds receive `OperationContext` consistently, including
approval, model, tool, memory, session and internal provider transport retries.
It is a local execution context, not a field serialized to a model/tool server.
Retain existing authority, trace and budget meaning; do not grant authority by
attaching a scope or operation context. Avoid duplicate lifecycle owners in
engine dispatch, transport, and service adapters.

Handler elimination and `?` affect escaping effects, not operation ownership.
For example, a handled console request remains registered and trace-visible
even if the enclosing callable's escaping row is empty. Handler-produced
operations inherit cancellation unless running in the restricted cleanup path.

Keep three dimensions separate:

| Dimension | Meaning |
|---|---|
| Run outcome | Engine success, language failure, or external cancellation |
| Operation evidence | Not dispatched, confirmed completion/partial outcome, or unknown external outcome |
| Local cleanup | Pending or settled, with cleanup failures reported |

Preserve service-specific details: bytes already written, confirmed memory
commit/version, process status, or remote acknowledgement. A generic cancelled
error cannot replace this evidence. Successful external work that races with
run cancellation must be recorded before the engine abandons its continuation.
Unknown remote completion may coexist with fully settled local resources.

## 6. Adapter And Cleanup Contracts

Each adapter documents whether pending work can be interrupted, only observed
to completion, or can leave an uncertain external outcome. Do not infer these
properties from error strings or from the Rust future type.

| Domain | Required integration |
|---|---|
| Console | Cancel the requesting wait without losing input or granting ownership of the process-wide stdin broker to one run |
| Stream/TLS | Coordinate pending-operation cancellation with stream ownership; if interruption corrupts framing, invalidate the stream explicitly; closing a shared stream is not equivalent to cancelling one request |
| Command | Retain supervisor ownership through termination/reap; report backend process-containment limits |
| Model/tool/approval | Stop local wait/retries, preserve request identities and response races; remote cancellation needs actual protocol evidence |
| Memory/session | Use bounded connection-owning backend execution; preserve structured commit/non-commit/unknown evidence and prevent late cancellation from affecting the next job |
| Filesystem | Coordinate blocking operations and resource lifetime; report partial writes, and roll back only where the adapter actually provides that guarantee |

Only futures with a documented cancellation-safe contract may be dropped as the
complete cancellation mechanism. `spawn_blocking` is not a cancellation API.
Caller cancellation cannot detach a database write or command supervisor and
erase its eventual result. Locks/busy waits/long database operations need their
own bounded or interruptible backend strategy, not just a token around an await.

Cleanup is protected from the already-triggered business cancellation signal,
but has a separate monotonic deadline and admission restricted to releasing
owned resources. It does not reset business budgets or authorize new work.
Record cleanup-caused external operations where relevant. If cleanup cannot
finish within its bound, retain ownership and report pending work; do not
pretend a non-cooperative thread/process was killed. An embedding application
owns the lifecycle of the supervisor used for outstanding Host work.

Cancelling a request does not establish rollback. Storage preserves a confirmed
receipt, confirmed non-commit or unknown outcome as defined in
[Bounded Versioned Storage](etas-storage-design.md#3-commit-evidence-and-recovery).
This evidence is independent of the run's cancellation status and local cleanup.
Do not infer it from `Result::is_ok()` or overwrite it with a cancellation error.
Unknown writes cannot be automatically retried without a verified idempotency
or reconciliation contract, and a missing/expired receipt alone does not prove
non-commit.

Database rollback/cleanup uses its protected deadline. Detach and drain the
job's interrupt callbacks before connection reuse; quarantine a connection whose
transaction cleanup is uncertain. Bounded queue capacity includes retained
payload bytes and waiting admissions, not only a limit on active workers. The
storage design owns backend algorithms and proposed public std projections;
this lifecycle module does not become a database or transaction manager.

## 7. Engine Control, Trace And Recovery

External run cancellation is an execution control outcome. It is not converted
to `Error<E>`, a performed `Cancel` action, a missing-handler diagnostic, or a
retryable provider failure. Ordinary `?` and error handlers cannot swallow it.
A service-local `StreamError.Cancelled` remains an ordinary typed service result
when the execution scope itself is not stopped.

Budget exhaustion keeps the SPEC budget-error semantics; the owner of that
budget scope cancels unfinished children and propagates the prescribed error
after settling their work. Do not flatten all deadlines, sibling failure,
race-loser cancellation and external stop into one language exception. Healthy
enclosing scopes may handle language errors under existing SPEC rules.

Engines add cooperative safe points for pure computation and bounded evaluator
quanta that let the executor run signal/deadline tasks. Host mediation points
alone cannot cancel a CPU-only loop. Long pure kernels must be chunkable or
have an explicitly bounded execution contract without acquiring host effects.

Trace records scope parentage, cancellation requested/observed, origin/reason,
action/request/attempt correlation, operation evidence and local termination.
Use monotonic time for elapsed time/deadlines and preserve existing payload
redaction. Cancellation is not a synthetic requested action in effect facts.

Checkpoint codecs persist durable machine/ledger evidence, never tokens,
supervisor handles, cleanup guards, or live registration objects. Resume
creates fresh invocation cancellation state and maps durable occurrence
identities to it. An uncertain boundary is not replayed as success or blindly
re-executed. Reject an unsafe checkpoint/resume with a precise diagnostic if
the engine cannot represent or reconcile the boundary. Update and validate the
schema when durable fields change; do not guess missing fields in old artifacts.

## 8. Implementation And Acceptance

Core lifecycle precedes Interpreter integration, which precedes CLI wiring.
Adapters and storage work can proceed against the shared contract in parallel.
Replace service-local invocation tokens/ad-hoc stop flags during migration;
retain distinct resource-lifetime cancellation (such as closing a shared
stream) only where it represents a different documented operation. Keep one
authoritative action/Host dispatch path and remove superseded bypass paths.

Acceptance must include:

- Deterministic state-machine tests for register-vs-stop, complete-vs-stop,
  concurrent repeated stop, child admission, drain, late waiters and waiter drop.
- Real adapter tests for blocked stdin, loopback TCP/TLS, command supervision,
  SQLite contention/commit races and pending approval/provider requests; inject
  lost acknowledgements to test uncertain outcomes.
- Public engine tests for CPU-only/deep execution, yielding on a single-thread
  executor, nested handlers, same-action independent invocations and retries;
  include pre-first-poll owner drop and distinguish it from observer drop.
- Partial I/O and already-committed writes retain their evidence after cancel;
  no successful cleanup claim is possible while owned work remains.
- Checkpoint/resume cannot restore a cancelled live token, reset consumed
  budgets, duplicate a completed occurrence, or retry an unknown write.
- CLI subprocess tests exercise actual signals and input; assertions cover
  terminal status, bounded waiting, process behavior, and structured output.

Use barriers/channels and injected clocks for precise race tests, watchdogs for
deadlock detection, and bounded subprocess tests for OS guarantees. A mock
returning `Cancelled` is not evidence that real I/O or processes were stopped.
Track live registrations and results so tests can detect leaked ownership.
Also apply the [storage acceptance matrix](etas-storage-design.md#10-acceptance-matrix)
for durable versions, real multi-process CAS, bounded data handling and typed
source-level receipt/unknown-outcome behavior.

## 9. Practice References

- [Kotlin Job](https://kotlinlang.org/api/kotlinx.coroutines/kotlinx-coroutines-core/kotlinx.coroutines/-job/): parent/child lifecycle and cancellation versus completion.
- [Python TaskGroup](https://docs.python.org/3/library/asyncio-task.html#task-groups): structured child ownership and waiting on scope exit; Etas keeps its own SPEC error/handler semantics.
- [Eio Cancel](https://ocaml.org/p/eio/latest/doc/eio/Eio/Cancel/index.html): cancellation contexts and pending-operation hooks at effect mediation.
- [Tokio Graceful Shutdown](https://tokio.rs/tokio/topics/shutdown): separate shutdown notification from waiting for actual completion.
- [Tokio CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html): notification and child propagation; not atomic subtree admission.
- [Tokio TaskTracker](https://docs.rs/tokio-util/latest/tokio_util/task/task_tracker/struct.TaskTracker.html): waiting for owned local futures, not remote completion or admission control.
- [Tokio JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html): task ownership, abort and blocking-task limitations.
