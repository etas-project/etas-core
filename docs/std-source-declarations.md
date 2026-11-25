# Etas standard-library implementation migration

## Active Agent declarations

`std.agent.Active` and `Autonomous` are declared in Etas and exported by the
prelude. Active membership is compiler evidence for nominal Agent declarations;
an explicit user impl cannot grant it to a passive type. Generic bounds and
role super-specs retain this evidence.

The frontend accepts `agent Counter = { value: i32 };` and keeps its private
state layout separate from the opaque reference type. Ordinary impl methods
retain Agent ownership and their `self`/`var self` receiver mode. Construction,
generic state fields, bound methods and external field rejection have frontend
checks. State layout contributes to the checked session fingerprint.

Source std 0.6.0 installs successfully with native schema 274, body schema 90
and compiler contract 42. The installed CLI checks a generic `Box<T>` Agent
using the prelude's Active bound, and rejects a passive type's explicit Active
impl. The three focused frontend checks pass, including extraction of both
read and updating request methods.

Checked method calls distinguish ordinary synchronous calls, Agent requests
and direct self helpers within an update. Helper evidence follows structural
HIR ownership and stops at Lambda/Handler boundaries. Modern Agent methods
retain protected method contexts and use ordinary impl method identities for
definition projection, rather than the legacy impl-agent header path.

Managed construction and requests retain Agentic.create/call in requested and
default action rows and ordered static traces. Their selectors are checked
type/method identities; these metadata actions have no user-perform payload
ABI. Selector specialization therefore uses their checked selector contracts,
without requiring an executable action signature. Agent state observations
remain non-deterministic even when the escaping Effect row is empty.

Interpreter 0.21.0 allocates scoped instance references and keeps their private
committed state in the scheduler. Calls submit child requests and suspend their
caller. Read admission captures a persistent snapshot; writers queue for one
exclusive pending state, retaining it across suspension. Normal completion,
including a returned Err value, commits before reply delivery. Cancellation,
escaping errors and enclosing Handler finish discard the pending state. The
trace records instance/request identities and admission/commit versions.

The installed CLI runs `active_counter.es` and `active_counter_suspend.es` at
O0 and O3. They verify shared aliases, bound read/update methods, independent
instances, reads during a suspended update, queued writers, commit-before-reply,
returned Err values, race cancellation and Handler finish rollback. A generic
Box<i32> also runs under both optimization levels. Counter's initial execution
milestone is complete; these checks do not establish the full Agent contract.
The installed active_request_context.es example also runs at both levels:
two concurrent callers install different Provider handlers for one Service
instance and receive 101 and 202 from their respective request environments.

Request contexts now own their authority and approval records. A task inherits
its caller's current grants; subsequent approvals do not spread to its siblings
or parent. Checked program scopes reuse that same task context. Agent admission
retains caller and owner constraints. Native boundaries in that request task
conjoin their grant predicates and sandbox capabilities and check each policy
separately. Receiver approvals do not become caller approval evidence. Console
dispatch refreshes authority after mediation.

Host 0.3.0 retains live caller and owner budget accounts. Admission deduplicates
shared accounts, preserves each account's earlier consumption and absolute
deadline, and reserves/settles/releases against all accounts atomically. Scope
observations report the originating account instead of summing duplicate usage.
The focused host checks and six interpreter context checks pass, including real
mailbox admission with two accounts and enforcement of an owner-only policy.
The installed CLI passes 16 runs across O0 and O3. The peer-cancellation entry
cancels a caller and its parked Agent request, keeps the other caller alive with
reply 202, and accepts a subsequent request with reply 303. Cancellation follows
the task lifetime tree across communication groups. Running children retain
their in-flight completion/error reporting; parked children are notified to
settle and native resource drains remain in force. All nine existing scheduler
checks pass, including primary-error preservation during child cancellation.
Composite account checkpoints are explicitly unsupported rather than flattened.

Evaluator context switches reuse the HIR revision's existing tree index. They
no longer rebuild the full index at every scheduled task or scoped call. The
installed verification executable has normal 0755 permissions and a verified
SDK receipt, so execution reuses the already solved standard-library data.

Delegated Handler authority/budget propagation still needs a complete audit.
Runtime policy/trace enforcement of managed Agentic boundaries, complete scope
escape checks/cleanup, construction-time injection, Autonomous startup, member
tools, erased roles, wait-cycle diagnostics and SessionLease remain unfinished.
Checkpoint capture of live instances/requests currently reports unsupported;
it does not persist references without the scheduler's state.

## File streams and acquisition

`std.fs` declares `FileMode`, the source record `FileStream<R>` with a private
connection capability, and its `ByteStream` implementation. `open`, `sync` and
`canonicalize` perform the corresponding source `Fs` actions. The host retains
OS acquisition, contained path resolution, file I/O and stable-storage sync.
ReadWrite acquisition checks both region grants and sandbox roots before open.
File reads and writes share one admission slot and file position. Close waits
for already dispatched file work; it does not imply fsync.

The interpreter preserves region and mode in a sealed file reference. Explicit
`Fs.open` handlers must resume with the requested region and exact mode. Native
standard impl owner bounds resolve against checked target declarations when
linking an SDK generic with a consumer region; portable canonical facts still
reject source-local bounds.

The normal SDK installation succeeds with native schema 272, body schema 88,
metadata schema 75 and compiler contract 41. Its artifact hash is
`blake3:7b1ba00399e0f7256f0c84b18f682ef6fe99908fe134e26b6ba88ae7c07ed28a`.
The current installed CLI reuses that checked SDK after consumer linking fixes.
`examples/spec_file_stream.es` completes real writes, sync, shared-position
read/write, handler resumption and canonicalization inside a resource scope;
the resulting file contains `heylo`. The existing 22 stream checks also pass.

This is not the complete file authority contract. Mode-dependent required
authority still needs checked summary propagation and language contract
enforcement, separately from requested actions and byte-transfer traces. A
pending native file handler contract also cannot yet cross a checkpoint.

## TCP/TLS values and configuration

`Host.parse`, `Host.to_string`, `Port.new` and `Port.value` are implemented in
Etas inside their owning impl blocks. Host fields are private. Parsing accepts
ASCII/punycode DNS labels and IPv4/IPv6 literals, normalizes DNS case and IPv6
spelling, and preserves an absolute DNS name's trailing dot. It does not resolve
DNS. Ports store a nonzero u16. `std.tls.Host` re-exports the TCP Host identity.

`DefaultTcpOptions` and `DefaultTlsConfig` are source values used by ordinary
default parameters. The host protocol carries their timeout, nodelay, minimum
TLS version, trust anchors and ALPN settings. DNS and all TCP address attempts
share one local deadline. TLS configuration is validated before taking ownership
of the TCP connection; the handshake retains the parent deadline and does not
fall back to plaintext. V13 offers TLS 1.3 only. Certificate and server-name
checks remain enabled. Bundle accepts PEM certificates or a DER certificate;
Platform uses the host's existing WebPKI root profile. Requested ALPN must be
negotiated. Source NetworkError/TlsError records expose the specified stable
codes and redacted messages.

The normal SDK producer checks and installs these declarations with native
schema 271, body schema 87, metadata schema 75 and compiler contract 40. The
artifact hash is
`blake3:7873b5d69fb8d77513526eb60cf56ec82f9ff20f65d30e357ca2febcf4af20ea`.
The installed CLI runs `examples/spec_network_config.es` successfully. A local
TLS 1.3 server also verifies `examples/spec_tls_echo.es`: the client trusts a
test CA Bundle, negotiates `etas/1`, exchanges ping/pong and closes the stream.
The existing 22 stream checks pass with explicit protocol defaults and the
current idempotent-close assertion.

File authority summaries, managed Source cursors and full resource escape/cleanup
summaries remain unfinished parts of the std spec migration.

## Resource scope source and runtime ownership

`std.resource.scope` now has a real Etas body. It invokes the private
`Scope.enter` execution capability, examines the returned source
`ScopeReport<R>`, raises the source `ResourceError.CleanupFailed` on cleanup
failure, and otherwise returns the body value. Rust retains scope admission,
child joining, native ownership and typed diagnostic materialization. It does
not duplicate the public cleanup-error algorithm.

The native scope summary also retains `ScopeReport.value` as the checked result
of invoking its zero-argument body callback. This relation uses the existing
`CallResult` parameter projection, including when R is a callable or an
aggregate carrying one. An `Any` result loses that provenance and cannot justify
invoking a returned function from its escaping Effect row alone.

Checked callback contracts distinguish a fresh resource boundary from an
ordinary drained task callback. The compiler derives the resource boundary
from the private scope role and direct source forwarding, then publishes the
callback slots for both flows and associated methods. Metadata schema 75 and
native schema 270 retain those slots through SDK publication and import path
relocation. This identifies the owner boundary; it does not by itself prove
that the callback result or an outer assignment is free of newly owned handles.
Both the published interface fingerprint and the importing project's dependency
fingerprint include these contracts. A boundary change therefore invalidates
cached checking even when the ordinary type and Effect signature is unchanged.

The whole standard library checks and installs through the normal SDK producer
with native schema 270, metadata schema 75 and compiler contract 39. An external cache-off run of
`examples/spec_resource_scope.es` through that installed CLI succeeds: an
unawaited child completes before scope return, its explicit outer Handler
updates the original mutable binding, and the body value is preserved. A second
scope returns a lambda capturing an immutable local; invoking it after scope
completion returns 42. The
installed SDK artifact hash is
`blake3:5b5193b916303730381cdf47c8dae89b210f827307f980064443c12be999f7cb`.

This is the scope execution batch, not acceptance of the full resource
contract. Compile-time escape summaries, implicit cleanup action/effect and
provenance accounting, filesystem-stream ownership, and restoration of active
explicit resource owners remain open. Active-owner checkpoint capture reports
unsupported rather than dropping the ownership graph. Completed scope values
are ordinary typed source data.

## Earlier migration receipts

The maintained std contains 122 Etas files and 600 bodies. A fresh strict
frontend220 build checks all 600 bodies for types and Effects in 20.763 seconds
with caching disabled and the complete legacy registry compiled out. The SDK
artifact hash is
`blake3:49ce97530bfbd68307115373e49b173e987a905b0aeacfb26494f0ba3225db41`.
A second independent build through `std install` produces identical manifest
and artifact bytes. The whole prefix is moved, and only the task-owned producer
source copy is removed. All 122 installed source files match the maintained tree.

Two independent external application sessions each pass all seven execution
entries and six sealed checkpoint restores. They cover collections, protected
filesystem I/O, FIFO/stateful/yielding scheduling, try_join cancellation cleanup,
and Handler/Lambda lexical control flow and original-var writeback. The matching
binary SHA256 is
`1b5aa2d37df5af3d071f090f298e2fc9deacc3e261828ae6e8832a6934c93ba6`.
The consolidated runtime suite passes 1,617 tests, with zero failures and one
ignored test. The matching installation receipt is
`/tmp/etas-source-matching227-delivery/receipt.json`.

The complete SDK installation integration test also passes. Adding a generic
API requires only one new Etas file; its private record and owning impl survive
publication and relocation. It verifies deterministic builds, removed producer
sources, external check/run, cold/warm caches, actual builtin std lock upgrades,
and cross-package execution. A source-only join body rebuild changes the result
from 42 to 43 through the original cache. The test log is
`/tmp/etas-source-sdk227-delivery-test.log`.

Sampled optimization preserves every native analysis and checked-publication
pass while reducing the same full-SDK application from about 60 seconds to
17.864 seconds. Indexed type lookup reduces summary solving to 11.193 seconds;
alias-summary joins reduce memory-provenance analysis to 0.926 seconds. CLI
check also reuses its selected provider instead of loading the SDK twice.

The wider module compatibility test identified a returned-lambda spec-receiver
metadata scope mismatch in `std.extra.reader`. Its repair uses each node's
ordered checked binding interface, including lexical captures. The current
compiler code advances frontend221 and metadata68; acceptance with that new
producer remains pending. The metadata suite passes 125 tests after the repair.
The wider module test, broader
consolidated regressions, final installed IDE/VSIX acceptance, provider rollout
and all-five-repository published Git/main delivery remain open. The normal
release provider remains legacy. Core implementation is published on its feature
branch; downstream dependency closure is being verified without path overrides.

The earlier complete strict frontend202
producer successfully checks 600 bodies for types and Effects and publishes the
SDK. Its checked `LibraryBodies` data is 93,128,555 bytes; the complete compressed
artifact is 5,939,408 bytes. Metadata67 gives checked bodies a separate 512 MiB
uncompressed allowance. Public sections retain their 64 MiB limit, and the
compressed section and artifact limits remain 32 MiB and 64 MiB.

The moved matching installation passes all seven execution entries and six
independent checkpoint restores. Cache/lock compatibility, performance, IDE and
publication acceptance remain open. The earlier subset SDK task application
passes checking after canonical flow signatures, Effect reexports and requested
action rows share checked source identities. Selected-entry execution also limits
task and updating-method proof materialization to its actual reachability; whole
library builds still check every body. Scoped existential values now retain
their original checked packing program and a checked projection chain. Their
interface and payload move together to the receiving program; its prepared
implementation witness supplies dispatch. The checkpoint format preserves that
origin. Native-owned Handler values now retain their actual checked program,
captured frame and projection chain. An internal expression boundary executes
the caller's body under the source Handler without inventing a forwarding flow.
Frontend211 prepares the source continuation carrier from the caller's actual
checked Perform results, including prepared generic instances. The complete SDK
FIFO application now completes both child tasks and its root callback, returns
42 without diagnostics, and writes two encrypted checkpoint files. Independent
Native Handler contracts, suspended
expression origins, checked program ownership, shared lexical parent frames and
source callback origins now pass their previous failure points. Pending calls
validate scoped callable types against actual invocation sites. Independent
FIFO checkpoint restoration passes with both the earlier and current bundle.

Frontend208 repairs target-owned spec bounds and avoids registering an existing
checked target declaration as a foreign shadow of its bootstrap ABI reservation.
The complete SDK's readonly filesystem application now returns 42 with no
diagnostics after an actual successful `Fs.read`, in 60.330 seconds with caching
disabled. The earlier frontend208 stateful policy application reached its task
scope and started its callback, then reached the Handler projection gap.
Frontend210's call-summary collection still missed direct caller requests;
frontend211 fixes that gap. The FIFO execution takes 61.153 seconds with caching
disabled. Its checkpoint labels are `20`, `22`; the public JSON report contains
observation summaries, while separate files contain the sealed resume payloads.
The same complete SDK also executes the stateful policy, yielding and try_join
fixtures with result 42, in 62.580, 60.972 and 66.340 seconds respectively. The
stateful checkpoint order is `22`, `20`. Try_join selects the expected typed
error and confirms cancellation cleanup. There are six sealed checkpoint files
across FIFO, stateful and yielding. All six now restore independently to 42 from
the moved matching installation, using automatic std discovery outside the
repository. Installed collections and protected file reading also return 42.

Frontend217 also reruns FIFO successfully against the complete frontend202
bundle, returning 42 and writing two checkpoints in 68.826 seconds. Internal
Handler expressions now preserve lexical return, loop control, resume and finish
payload types. Runtime221 closes crossed Handler/Retry scopes for loop exits and
restores the caller's lexical finish after closing an internal Handler boundary.
The combined scope fixture returns 42. Its strengthened version also verifies
that Handler and Lambda assignments update the original outer var bindings, and
returns 42 against the current frontend217 bundle. The complete moved-installation
run also passes try_join cancellation/cleanup and this scope fixture. Its receipt
is `/tmp/etas-source-matching-install221-acceptance/receipt.json`; all seven entries
return 42, and all six sealed checkpoint restores return 42.

Source Map constructors, annotations and methods now use the same checked
declaration identity. Generic implementations also retain their own canonical
owner, parameters and constraints, independently of whether they contain methods.
The published `WorkspacePath<R> ~ CommandDirectory` marker retains `R ~ Region`
and a canonical parameter reference. It is not also exported as an unparameterized
type satisfaction. The existing implementation codec roundtrip passes.

The complete SDK collections application passes both type and Effect checking.
Its priority queue retains the checked origin of its stored comparator through
construction and method applications. Already applied method return recipes reuse
their captured argument projections. Native callable specialization now retains
implementation owner arguments. Enum reexports also retain their checked
declaration identity: `std.prelude.Option` and `std.option.Option` no longer
create separate constructors. With frontend205, the unchanged external application
using source `join`, `Map` and `PriorityQueue` completes with result 42 and no
diagnostics against the complete frontend202 SDK. Actual execution takes 64.640
seconds with caching disabled. The protected-I/O application also passes with
frontend208. All four Task execution modes also pass with frontend211.
Checkpoint restoration, general Handler lexical exit propagation, moved-prefix
offline and IDE paths are not yet accepted.

Two independent complete frontend202 builds produce the same artifact hash:
`721ca20288567ed0409eb0ee41506d78e4aab6f75b021e02f4b95a75ae4fdcac`.
Member candidates and same-path action overloads have deterministic ordering.
The actual `std install` command succeeds, installs its matching executable, and
the whole prefix has been moved. Moved-prefix offline execution and source-copy
removal are still pending runtime acceptance.

Source public values execute their checked Etas initializer. The interpreter
selects the initializer's existing item plan, preserves the calling task and
Handler context, and bridges the resulting value to the application's type
revision. No synthetic forwarding flow or Rust singleton replaces that body.

Lambda and Handler captures share the original outer `var` within its declaring
lexical scope. Synchronous `run` / `Scope.enter` preserves the logical owner;
independent `start` tasks cannot receive a callable retaining that binding.
Checkpoint100 retains the checked shared-cell graph, native Handler metadata and
ownership checks. Earlier
capture, task and checkpoint results below remain distinct from installed SDK
application acceptance.

The existing SourceBootstrap iterator integration passes actual Etas collections,
math, filesystem wrappers and 16 checkpoint restorations. Its task integration
passes FIFO and stateful source schedulers, yielding, six checkpoint restorations,
typed error selection, sibling cancellation and matching cleanup events. The
existing concurrency/checkpoint/CPU and scheduler-snapshot union reports 316
passed, 0 failed and 1 ignored. These are not full installed SDK results.

A minimal 13-part SDK passes independent-process artifact equality, relocation,
default provider discovery, an Etas-only added generic API and a source-body
change from result 42 to 43. Cold and warm application runs with a populated disk
cache also observe the rebuilt body. The timings do not establish incremental
parse reuse or a speedup guarantee.

Strict development and CI builds select artifact mode with `forbid-legacy-std`.
Source build/install commands bootstrap without first loading an installed SDK.
The normal release default remains legacy pending complete acceptance. Delivery
still requires the complete SDK's external Task execution,
moved offline installation, lock upgrade, independent compatibility sessions, checkpoint/resume,
IDE navigation and packaging, published dependency revisions and main merge/push.
Current frontend cache schema is 220; the accepted complete SDK was built with
frontend217. The library uses libraryHIR68 and package metadata67,
descriptor graph19, native query14, compilerRoleABI29 and checkpoint100. The
complete SDK collections runtime passes; scoped existential transfer and the
remaining installed SDK runtime acceptance are unfinished.

The following sections retain earlier migration history. Their build counts and
schema versions describe those earlier checks.

## Returned callable positions

Checked return summaries now retain a map from typed value positions to their
callable origins. Tuple slots, record fields, enum payloads and collection
items stay distinct through bindings, assignment, closure capture and argument
substitution. A cursor's yielded element and its returned tail are separate
positions; neither borrows the initial next callback's proof.

Portable callable graphs retain these positions both at the declaration's
return and at each returned callable's return. Metadata decoding, reference
validation, canonicalization, fingerprints and budgets include both maps.
A checked `CallResult` step selects the output of a zero-argument callable;
calls with arguments use their actual checked argument bindings. Unavailable
origins remain unavailable. A checked empty payload remains distinct from
missing evidence.

The 16 existing callable integration cases pass, including a delayed generic
array lookup that returns a callback. The existing binary metadata replay case
also passes with independently returned record callbacks and eight resolved
Console events; the consumer checks only its own source. The corrected strict
CLI release passes and its complete source build reaches 10/4 diagnostics. Current schemas are
frontend176, libraryHIR64, package metadata62, native query14,
compilerRoleABI28 and checkpoint94. This checkpoint update belongs to the
following dynamic-loop batch; the completed graph build used checkpoint93.

## Source declarations for runtime defaults

An argument-free `@default_handler` marks an action with a runtime default. Its
Effect suffix declares the effects that default may propagate. An explicit
`![]` declares a default with no escaping Etas effects; an unannotated action
has no runtime default declared by source.

```etas
public effect Console extends FileIO {
    @default_handler
    action stdout_write(text: string) -> unit ![Error<std.io.IOError>];
}
```

Source now owns 48 such declarations, including pure Memory reads/writes and
the `Remaining` Effect row of Async.await. Source signature checking and
metadata loading read their checked rows directly, without reconstructing
them from native registry declarations. Action identity, signature and the
optional default row travel together through package metadata; presence of an
empty row is preserved. Current schemas are frontend176, libraryHIR64,
package metadata62, native query14 and compilerRoleABI28.

An explicit matching handler consumes the request first and removes its
conditional default effects. Unhandled source Console actions enter the
existing Console host boundary, which retains permission, budget, trace and
replay processing. The runtime checks their scalar ABI and Error<IOError>
contract. Host errors decode into the source-owned IOError representation.
Defaults without a registered runtime implementation report a missing-handler
error. Declaring a default does not implement that missing host primitive;
full module execution remains a release gate.

The current bootstrap rewrite removes the `StdSourceRole::ALL` declaration
factory and the handwritten type, enum, opaque, record and projection reference
tables from the compiler contract catalog. Source HIR now generates ordinary
type/spec/enum constructor heads, including authored type/effect binders and
generic bounds. Source aliases continue through normal alias checking; the
kernel does not create a second alias graph. Source prelude bindings come from
resolved source declarations and imports. The kernel retains language scalars,
native Effect execution rules and explicit compiler/host primitive contracts.

Compiler data roles retain canonical execution identities while their records
and enums belong to source. Their ordinary fields and constructors are no
longer required to be opaque native declarations. The installed query publisher
preserves these identities from the selected source bundle. The native ABI
fingerprint excludes ordinary source type heads, so adding an ordinary `.es`
declaration does not itself change the toolchain ABI. Source Limit payload
checking selects the actual checked enum instead of looking for a type role
annotation. Current schemas are frontend176, libraryHIR64, package metadata62, native query14 and compilerRoleABI28.

Source queries now retain declaration visibility. Private types, enums and
specs are projected from the compiled bundle with their real definition keys,
fields, generic binders and variants. Type-dependency lowering can resolve
them; public qualified lookup, module exports and prelude publication exclude
them. Private enum constructors inherit the owner's visibility. This supplies
retained compiler/evidence fields and recursive collection representations
without a handwritten private-type table or exporting private declarations.
The registry image schema is now 14.

Source Option/Result constructors now retain their ordinary source enum result
identity; the legacy provider keeps its intrinsic constructor ABI. The enum
validator selects that distinction from the source declaration binding.

The minimal compiler contract query stages ordinary constructor heads from
source HIR. Normal type checking then supplies the source action signatures,
including inferred signature generics and overloads. The completed query goes
through full registry validation before publication; registry image capture
also validates. The native contract factory does not resolve source-owned value
types before their declarations exist. Handler rows and existential bounds are
represented explicitly in the query and metadata, including Effect arguments
of ordinary source constructors.

The source authoring pass has reached its consolidation point: 122 source files,
no std.binding annotations, and the remaining 25 declaration-only flows are
20 explicit execution mechanisms and five spec methods. Consolidated Rust
workspace compilation has passed in Core, Frontend, Interpreter, Main and IDE.
The CLI also builds with forbid-legacy-std. Cache-disabled whole std builds now
reach normal source parsing, name resolution and type/Effect checking. The
premature ApprovalRequest<T> validation failure has been removed by staging
source signatures. Declaration checking previously produced 183, 69, 36 and 24
entries. Source flow overload grouping, checked input-based selection and stable
per-implementation identities now pass that declaration gate. Re-preparing the
Effect registry after typed contextual-constructor rewrites fixes the stale HIR
snapshot. The next full build completed with 575 Effect/execution diagnostics
(428 unique location/message pairs), dominated by 285 loop-limit constructor
reports and 156 updating non-suspension reports. This run predates the source
Limit constructor and selected-overload suspension fixes now written. Whole std
build success, completed-query ABI validation, independent bootstrap and full
runtime/SDK acceptance remain pending.
Source algorithms and release delivery are not accepted on the basis of Rust
compilation or the structural inventory.

Generic suspension analysis now uses checked type applications as proof nodes.
It resolves concrete spec witnesses through the existing method index and keeps
unresolved generic witnesses unknown. Recursive applications use a monotone
worklist with execution checkpoints and the compiler graph node budget. The
existing source-call suspension test now includes an updating caller through a
generic spec method; all 16 callable-application tests pass. Whole-library
checking falls from 274 diagnostics (249 unique location/message pairs) to 220
(195 unique pairs), including 25 updating declarations and 77 updating calls
still requiring a proof. The earlier limit and action ABI fixes account for the
575 -> 293 -> 274 progress. These are failed-build diagnostics, not accepted
library artifacts. The current proof run takes about 247 seconds; about 240
seconds are spent in resource alias provenance.

After that run, callback contract lookup was extended to homogeneous collection
payloads, and primitive indexing proofs were changed to use the actual checked
generic type application. Their whole std build completed with 189 diagnostics
(164 unique location/message pairs), including 19 updating declarations and
52 updating calls still requiring a proof. That run remains a failed build.

The next analysis batch records a source cursor's stored next origins separately
from its factory call. Suspension proof nodes follow both the initial callback
and callbacks in returned tails, retaining captured type applications and the
checked zero-argument step ABI. Missing callable evidence stays unknown. The new
facts are preserved in body artifacts and library images; replacing their
expression invalidates them. The existing 16-test callable-application suite
passes, including a generic recursive cursor instantiated with i32. The
whole-library check for that batch completed with 168 diagnostics (143 unique
location/message pairs), including 16 updating declarations and 34 updating
calls still requiring a proof. It remains a failed build. Implicit step Effect and
callable-element provenance integration, dynamic limits and remaining host ABI
integration remain open.

Generic return-origin analysis now preserves known empty aggregate payloads.
An early empty enum variant no longer discards callable origins returned by
another branch. Known empty return sources remain distinct from missing return
evidence, missing control information and functions with no observed return.
All 16 existing callable-application cases pass, including a generic Array<T>
method with an early empty enum return followed by invoking a returned callback.
All four existing return-origin checks pass, including the handler finish rule.
The whole std check after that fix still reported 168 diagnostics (143 unique
location/message pairs).

Source enum constructor import aliases now resolve to their canonical source
declarations before collecting payload origins. Implicit expression and final
block results retain the same checked callback origins as explicit returns.
The whole std check with those fixes completed with 138 diagnostics (113 unique
location/message pairs), including five updating declarations and 15 updating
calls still requiring non-suspension evidence. This remains a failed build.

A further structural-result fix preserves known empty generic match branches
alongside branches that explicitly return callback payloads. The existing
16-case callable-application suite passes with an actual array alias, a generic
Index-like spec, nested match returns and imported enum constructor aliases.
The same suite covers an expression-bodied recursive source cursor. The whole
std check with the structural-result fix completed with 117 diagnostics (111
unique location/message pairs). The Array.get callback match origin reports are
gone; five updating declarations and 15 updating calls still need a proof. Full
SDK, runtime and delivery acceptance remain pending.

Suspension instances now retain checked callback arguments crossing source
call boundaries and actual dynamic call origins. Capture environments carry
generic and value bindings into these instances. Recursive parameter
substitution uses a worklist, and the proof graph charges retained binding
storage to its existing compiler budget. Missing, cyclic and frozen invocation
evidence remains unknown. The new facts are preserved in body artifacts and
invalidated when their expressions change.

Method-evidence metadata now binds a lambda's own input symbols in its callback
argument graph instead of using the enclosing flow's parameter list. Descriptor
validation follows the actual nominal/record/enum callback carrier and its
generic bindings. All 16 existing callable-application tests pass, including a
generic record storing a closure that invokes a generic spec method, a concrete
updating caller, and metadata generation. The whole std check with this batch
still reports 117 diagnostics (111 unique location/message pairs). Its remaining
updating paths reach a source iterable whose returned element and tail have
flattened origins: the collection's comparison callback is incorrectly included
among the tail cursor's next callbacks. Field/tuple-sensitive origin projection
is required to complete that proof.

Overloaded inherent methods retain separate checked MethodIds. Their shared
static spelling requires method selection, so valid associated constructor
overloads can publish metadata without a canonical-path collision. The existing
16-case suite now includes both constructors of a generic record, a closure
calling a generic spec method, an updating caller and metadata publication. All
11 existing type-implementation identity tests and nine trace-descriptor tests
also pass. Full installed SDK and runtime acceptance remain pending.

Checked callback descriptors now carry an explicit value projection path for
record fields, tuple positions, enum payloads and sequence/map positions. The
path participates in trace fingerprints, behavioral labels, proof budgets and
binary metadata. Its type resolver keeps each applied declaration's arguments
in their original binder scope. The existing descriptor test now covers an
alias of a generic record containing heterogeneous callbacks, scalar argument
checking, graph relocation and distinct path fingerprints. All nine descriptor
tests and 16 trace-inclusion tests pass; an existing metadata roundtrip preserves
all path-step kinds. The source ParameterCallInfo and TraceLabelInfo declarations
now expose the same path, and the interpreter projects every path step into
those checked source values. The complete CLI check and forbid-legacy-std
release build pass for this reflection transport batch.

The next native analysis batch selects the returned cursor's callback through
Some.payload[0], tuple[1] and its checked next field, before joining origins with
the yielded element. This uses checked constructor identities and exact value
positions; it does not discard callbacks by arity or assume an empty Effect row
proves suspension behavior. Cursor-tail evidence remains separate from normal
return observations and control gaps. Generic row/type substitution preserves
it, frozen relocation rebinds its source identities, and typed-HIR replacements
invalidate it in affected owners. The existing cursor case now covers a generic
record containing both an element and a comparison callback; the existing
16-case callable suite passes, along with the existing scoped-index case (ten
actual indexes) and four existing return-source cases. The final strict CLI
release build succeeds. Its whole std check reduces diagnostics from 117 to 97
(111 to 91 unique location/message pairs), with no new diagnostics. All five
remaining updating declarations and 15 updating calls now have their checked
non-suspension proof. The build still fails the remaining Effect/execution gate.

General source latent value environments and returned aggregate metadata still
use flat origin sets. Their structured placement/selection graph, heterogeneous
parameter projections and implicit cursor-step Effect integration remain to be
implemented. The current 97 diagnostics include the Task heterogeneous tuple
callbacks, source cursor environments, default Handler boundaries and dynamic
limits. The whole check took 365.13 seconds, including 354.63 seconds in memory
provenance; that analysis has not been weakened to reduce the check time. This
does not establish whole-library or SDK acceptance.

Source flow overloads retain their actual source symbols and bodies. Canonical
implementation keys derive from checked generic input contracts, rather than
arena IDs, file positions, output types or wrapper names. Flow signatures and
Effect summaries transport these exact keys. Installed query data carries a
separate checked body for each overload; type selection, generic preparation,
Effect lookup and interpreter dispatch now retain that selected identity. A local
language example returns 42 across ordinary, named/default and generic tuple
calls. It uses the legacy provider solely for local language verification.
Installed overload execution has not passed: a small source-only SDK attempt
currently fails because the completed execution query references SessionId when
that source module is absent. This is not full-library acceptance.

Effect-action overloads retain a common request name and separate source symbols
and checked signatures. Candidate selection uses these source signatures in the
same order as the completed execution query. Source aliases do not collapse the
individual declarations. The SDK provider preserves the checked image's action
group instead of overwriting it for each metadata signature. Compiler check and
compile actions check their actual retained descriptor family and generic result
ABI separately for each overload. The latest source build eliminates the prior
action-name/signature diagnostics; SDK execution and the completed-query gates
remain unverified.

Conditional source implementations use explicit owner parameters, for example
impl<T ~ PromptEncode> Array<T> ~ PromptEncode. Their constraints are retained
as owner evidence rather than extra method generics. The selected source Index
and Number specs retain canonical identity without type-role annotations.
Source range literals construct the actual std.collections.Range record through
normal constructor checking and execution. Task's row argument participates in
the existing Effect-row solver after value-type inference, and StorageNonce
accepts the selected source Result declaration. These paths pass the latest
whole-source check through their previous diagnostics; runtime acceptance is
still required.

Execution observation roles now also bind current_session and tool.reference.
The former reads the actual active session into the selected source SessionId;
the latter retains the compiler's declaration-only operand check and resolves
identity/revision from the real frozen tool catalog. Public ToolRef descriptions
do not authorize tool registration or invocation.

MessageContext.capture and MessageContext.record are private associated
execution primitives on the private source context record. Capture supplies
the existing checkpointed message ordinal, current session, wall clock and
trace. Message.new still assembles identifiers, message fields and provenance
in Etas. Record translates the source-assembled message into the real workflow
event ledger without projecting or rewriting its payload. Their std-build ABI
checks use the actual checked context, Message<T>, Role, Time and optional
identity fields.

PayloadRepresentation.convert now has a private associated execution role,
bringing the execution set to 20. The std build checks its two type binders,
source argument, actual Option<T> result and closed Effect row. The VM uses
materialized source/target types, existing host representation and checked type
decoding, then constructs the actual source Some/None envelope. Message.cast
still handles message fields and metadata in Etas.

The conversion inspects declaration templates and their actual arguments
separately, so recursive generic ADTs do not require infinite monomorphization.
It shares capture's neutral identity gate and rejects execution/capability
types through the whole input/target graph, including empty containers and
opaque evidence. Ordinary private record fields do not by themselves grant
authority. The host projection now also offers a bounded entry that charges
the current value node/depth/byte budget before payload allocation; the existing
unbounded API retains its projection behavior. Cancellation/time boundaries
are preserved around conversion. Rust compilation has passed; their runtime
acceptance remains unverified.

The installed query publisher now projects ordinary source aliases directly
from their checked package metadata. Alias targets use the existing normalized
type projection, including structural Array<T> and nominal SchedulerHandle;
they are not nominal record storage or compiler-authored API declarations.
The query distinguishes source Alias from structural compiler Support contracts,
so native support templates retain their existing restrictions. Aliases require
a checked declaration binding and target; ordinary import lowering retains
the normal alias/generic semantics. Native query image schema is now 13.
Installed execution and private-type closure are still unverified.

Compiler action input and compile-handler result guards now read the real
retained definition capability from the selected source record. The runtime
keeps backend reference resolution, program ownership and frozen-contract
provenance checks; descriptive identity, signature and revision fields do not
authorize compilation. Private capability values have corresponding typed
runtime validation and cross-program projection paths. Source-build ABI
diagnostics require the shared private opaque retained field in AgentDef and
CallableDef, without copying their public API declarations into Rust.

Compiler report/descriptor projection now converts internal optional markers
to the actual source Option enum at typed record and enum payload boundaries.
Generic source records use their materialized field types. Direct optional
metadata fields use the same conversion. The source Some(T)/None payload
contract is checked at std build and at native projection. These are data ABI
operations; std algorithms remain in Etas.

Reflection's six successful paths now marshal native definition and metadata
observations into the selected source records/enums. Private retained fields
hold real definition leases, while public descriptions and readonly method
observations retain their actual revision, analysis availability and provenance.
DefinitionObservation is built from canonical compiler snapshots; the diff
algorithm continues to compare these values in Etas.

SyntaxBlock now owns a private RetainedSyntax field. Native AST transport reads
that field and never recreates a lease from public text or token descriptions.
Compiler lexer observations preserve body envelopes, including trailing
handlers; canonical text is a compiler observation. The public format flow
continues to run its Etas token-formatting algorithm. Private AST values keep
their originating native backend across checked program projection.

Three execution identities now bind parse_body and the AgentDef/CallableDef
with_body methods. Source build checks their closed Effect row, generic owner,
result and private carrier layouts. The VM parses into the actual source Result
and SyntaxBlock, and with_body edits a genuine retained definition snapshot
before publishing refreshed source data. Parsing, observation and editing use
the current run's cancellation and syntax/time limits. These new paths are
compiled as Rust, but not runtime tested; they are not accepted compiler
execution, installed private-type closure, checkpoint persistence or delivery.

With_body transfers the retained AST to the definition's originating native
backend through bounded retained-body export/import. The transfer preserves
the actual body form and provenance; imported syntax still grants no callable
environment or execution authority. Invocation parse and retention ceilings
are intersected with backend limits. Cached imports must satisfy the current
parse ceilings, and stricter retention ceilings reuse the normal accounting.

External type import now reserves the generated source constructor identity
and completes its representation through normal checked package metadata.
Compiler roles alone no longer force source records down the legacy opaque
import path. Actual intrinsic-only native declarations retain that path.
This preserves CallableDef's readonly identity while allowing private source
nominal dependencies in its record fields. Full installed closure still needs
consolidated verification; no second handwritten type table was introduced.

These changes are written and formatted, not compiled or tested. They do not
prove independent bootstrap or delivery. The primitive input/output bridges,
required carrier ABI checks against complete source layouts, contextual
prelude publication, remaining default-aware invocation/Effect analysis and
the full installed/offline/runtime/concurrency release gates remain open.

The current source batch replaces annotation-bound types with native Etas
records, enums and recursive types. All 88 `std.role` annotations on types,
enums and specs have been removed. There are no public opaque type declarations.
`List<T>` is defined directly as `[] | T :: List<T>`. Arrays use contiguous
language array storage, declared as `public alias Array<T> = [T];`. Map, set,
slice and iterator declarations remain source-owned.

Compiler descriptors and model/schema data now have explicit recursive data
shapes. Trust, task, continuation and host-resource values have source record
wrappers. Fifteen private opaque declarations remain solely for actual live
VM, OS, retained-compiler and security capabilities. They are not ordinary
collection or metadata representations. Descriptive identities are ordinary data
and cannot replace those capabilities. The twenty opaque declarations of
language scalars were removed; these names belong to the language.

This definition batch has not been compiled or tested, as requested. The new
empty/cell constructor-sum syntax now has AST/HIR, nominal layout lowering,
contextual construction, recursive pattern binding, exhaustiveness and iterator
support written in the compiler/runtime. The layout comes from the declaration,
regardless of its name; owners keep distinct nominal identities. These changes
are uncompiled and untested. This is the native sequence constructor form, not
general structural union subtyping.

The array-tree prototype was removed following the user's correction: recursive
ADTs are used where the structure is recursive, such as List and OrderedTree.
Array is a contiguous indexed array. Its methods are implemented in Etas using
array value operations; the runtime retains the Vec-backed value representation
and memory access rules. ArrayNode, rotations, tree paths and the array literal
forwarding to from_list have been deleted. The new `[T]` type expression lowers
directly to the existing language array representation. Array literals and empty
arrays use that representation, including ordinary constant arrays.

Map/Set literals still lower to source from_entries/from_list methods, and native
List literals retain their own source identity. The provider is selected once
in HIR. The earlier source push/extend rewrites remain; Array's own method
bodies use the language's array operators directly.

Source `for` lowers to an ordinary `iter()` method call. Collections, bytes and
all twelve integer range types have source implementations returning a native
`Iter` record. The execution ABI checks that record's `next` flow and the actual
source Option enum layout, and calls the closure through normal execution.
Collection traversal and range advancement stay in Etas. Source mode requires
this checked cursor protocol; it cannot fall back to legacy native iteration.
Option/Result/Iter identities are resolved from the selected source namespace,
including compiled metadata, without type role annotations or replacement
signatures. These new paths are written but uncompiled and untested.

Contextual empty arrays directly construct language arrays. Empty Map literals
select their actual source `new()` method through normal method checking. Literals whose type comes from a generic
call are deferred until those constraints solve; their constructor constraints
then go through the ordinary solver. Final type processing rewrites these
nodes to regular HIR calls before effect, reachability and execution analysis.
Synthetic type references preserve the selected nominal identity and source
locations. No storage values or substitute bodies are created by this rewrite.
Compiler-generated collection calls retain a literal-origin marker, including
for top-level constant classification. These paths remain uncompiled/untested.

Postfix `?` now produces the selected source `Result<T, E>` enum. Its result
type is materialized with the caller's generic bindings and retained across
suspension. Array indexing raises the selected source `IndexError` record with
index and length fields. Legacy provider values retain their earlier format.
The checkpoint stores the specialized result type in the try continuation.
These execution changes are written but have not been compiled or tested.

Compiler-marked Map/Set literal constructors in global constants now execute
their ordinary checked source methods on the VM. They share the caller's budget,
cancellation and execution state, and preserve checked program ownership through
cross-program calls. Recursive constant evaluation carries cancellation as its
own outcome. The pure boundary permits locally caught typed errors and rejects
escaping effects or suspension. Resource selector projection remains static and
uses the existing serialized schedule when a key requires computation. This
batch has only been formatted and reviewed as source; no execution is accepted.

Task, Scheduler and Continuation now have a written bridge between their
source record layouts and private live VM capabilities. The compiler records
the three selected public identities and derives each capability type from its
private field, without needing to export private type names. The new ABI check
compares the declared generic kinds, private record fields and distinct private
opaque capability types. No type role annotations have been restored.

Source task parameter projections are used by task row refinement, Effect
specialization, continuation binding and scope lifetime checking. Scope checks
also follow record representations and enum payloads. Runtime scope entry,
handler continuation binding and task creation construct the source records;
transfers read their private live handles. Value validation, cross-program
bridges and task checkpoint checks have matching paths written for these values.
Data.capture also uses the source Result envelope and excludes live execution
state. These changes have not been compiled or tested.

The source Task.start and Async.await ABI paths are now written against the
selected Task record and its result/effect arguments. Task.start has a verified
execution-role declaration rather than a competing bootstrap flow contract.
Ordinary run remains an Etas body, with its former invalid role annotation removed.
Saved task origins distinguish the source primitive item from legacy intrinsic
expressions and retain the specialized callback and Task result types.

Private Scheduler.next_completion and Scheduler.cancel now have execution-role
declarations and VM paths written. The runtime publishes actual terminal task
receipts in scope order, atomically observes/registers the completion consumer,
and transfers that wait while an inherited Handler executes. Completion queues,
sequence counters, wakeups and continuations are included in checkpoint format
v89. Source algorithms still choose and combine tasks; cancellation uses the
existing execution cancellation/drain mechanism.

Scoped-callback inference is now written for ordinary source wrappers, including
run. It starts at the actual fresh-scope entry and follows all uses of callback
parameters. Direct calls must be inside a managed callback; forwarding must go
to another inferred scoped parameter without retaining it in an unknown closure.
Recursive call cycles do not establish a promise by themselves. No ordinary API
name or user annotation supplies this contract.

The inferred callback slots and parameter names feed task lifetime and Effect
facts. Public metadata carries scoped-callback contracts by exported item path
or canonical method identity. Package serialization, dependency alias mapping,
frontend import validation and checked method argument binding have matching
paths written. Package metadata format is now 54. Function aliases resolve to
the existing callable identity; ambiguous signature-stage method selection does
not invent a scope proof.

The installed provider now generates an execution-role query table from Runtime
nodes in the compiled library, keyed by canonical callable/method identity.
Registry image format 9 retains this table. Imported source flow bindings and
method identities receive these facts for task ownership, continuation use and
Effect scheduling analysis. The table supplies no ordinary API signature or
implementation; those still come from the source bundle.

All nine combinator scope entries now call the source-owned Scope.inherit method.
Its body installs the source Handler and requests a fresh scope with an inherited
policy. Scope.enter has a third explicit bool parameter for this context rule;
the VM copies the active checked policy state, or uses the source-supplied
DefaultScheduler when none is active. Explicit public run uses its supplied
implementation. Task selection still calls the source policy's select method.

The latest design's default parameters are a remaining compiler prerequisite for
run and host command declarations. Parser/AST and declaration-site HIR storage
are now written, including the trailing suffix and receiver/anonymous-flow
syntax rules. Each default now has an independent HIR body and semantic/Effect
unit. Declaration-only flows, agents, tools, action declarations and spec methods
schedule type checking for their defaults. The written checks enforce the
declared type, definition-site binding rules, determinism, absence of actions
(including default-handled actions) and proven non-suspension. Spec implementations
cannot declare or replace defaults. These checks are uncompiled and untested.
The compiler now retains a separate parameter call interface on source callable
and action signatures. Its shared binder records supplied argument positions in
source order and actual source-parameter references for omitted defaults in
declaration order. Flow/agent/action calls and method selection use that mapping
for argument hints and generic/type/Effect-row constraint solving. Method facts
retain omitted-default references; spec implementations inherit their spec's
interface. Ordinary arrow values retain full arity. The earlier method-only
required-argument binder was removed. Checked library and cached flow/tool
signatures retain these interface facts. This is written, uncompiled code.

Source expression calls now have a written VM preparation stage. Supplied values
retain their checked parameter positions; omitted defaults are evaluated through
ordinary pending expressions and continuations in their definition scope. Earlier
bound parameters and checked generic bindings are supplied to that scope. Spec
defaults use their spec's parameter origins and type arguments. Only completed
argument lists enter a callable body or dispatch an action. Updating receivers
are acquired after default evaluation. Ordinary positional calls reuse values
without rebuilding an already ordered argument vector.

Checkpoint v89 includes the preparation cursor, bound positions and values,
default origins, invocation destination and caller frame. Its codecs and checked
snapshot validation are written. These execution/checkpoint paths are uncompiled
and untested. This was the local preparation format; the current cross-program
work below uses checkpoint schema 92. Default-aware invocation/alias/Effect
analysis, CLI entry defaults and model-tool defaults remain unfinished.

Named callable descriptor parameters now carry an optional portable default
reference. Its owner is a canonical declaration, method, spec method or action;
the reference records the parameter position and an authored revision fingerprint.
The source projector fingerprints the actual expression and referenced source
definitions/bindings, without absolute paths or compiler arena IDs. Unrelated
function bodies are not included merely because they share a source file.
The signature fingerprint includes these references. Descriptor wire schema 17
and package metadata schema 56 preserve them; older descriptor feature versions
continue to represent signatures without defaults.

Authored defaults now have a canonical expression-unit index keyed by the full
ParameterDefaultInfo (owner, position and revision). Each defining-program unit
retains its parameter, owner item, expression, lexical scope and declared output
type. This covers declaration-only flows as well as spec/action signatures; it
does not invent a flow body for a default expression. Library HIR schema 46 stores
the index alongside its defining HIR and types. Restoration rebuilds the index
from the restored checked program and compares it before publication.

The interpreter prepares the same index per program. Local default evaluation
now selects its expression and scope from that prepared table instead of
searching HIR scopes on every invocation. Installed std callable preparation
requires the matching authored default revision to be present. This is unit
retention and local execution integration. The imported callable path below is
written but has not been compiled or tested.

A checked default expression can now be selected as its own native execution
root. Selection reads the actual ParameterDefault Effect unit and requires its
deterministic, action-free and non-suspending evidence. It does not inherit the
owner callable's body summary, and it is not exposed as a first-class flow value.
Prepared std programs can select that root directly from the canonical default
reference, including defaults on bodyless action/spec declarations.
The evaluator entry prepares the defining lexical frame from native generic
bindings and the supplied parameter positions, then schedules the actual
expression on the normal VM. It shares frame preparation with local defaults;
receiver-update acquisition still follows successful default preparation.

Execution reachability now includes expressions for omitted Source defaults as
well as called items. Intrinsic dispatch and host-service discovery walk those
expression roots, including nested defaults, without visiting them repeatedly.
Residual coverage keeps transitive obligations tied to the reachable callee
scopes rather than inventing an owner scope for a default expression.

Checkpoint schema 92 records a default execution root by its canonical
ParameterDefaultInfo, then resolves it in the restored program's prepared unit
table and selects its checked plan. It does not export the defining parameter's
arena ID as the portable default identity. The expression boundary and installed
std projection are written; complete import coverage and checkpoint roundtrip
execution remain incomplete or unverified.

Local default evaluation now enters the expression through ProgramExpr and the
existing program enter/return machinery. The expression invocation owns its
actual code, canonical default reference, source call site, defining generic
bindings and preceding argument positions; it has no CallTarget or first-class
flow type. Caller-only receiver positions are normalized to declaration slots.
The normal pure boundary, caller continuation and shared invocation state remain
in place, and receiver update acquisition follows all defaults.

Checkpoint schema 92 distinguishes callable and expression program entries.
Capture/codec/restore branches retain pending expression entries and active
program-return frames. Restoration reselects the authored revision and derives
bindings from the checked caller frame. A pending entry reuses its restored
boundary invocation so it retains the same selected plan. Frames are restored
before a pending program entry, and already-projected arguments are restored in
the destination program's type universe. End-to-end checkpoint execution has
not been verified.

Imported callable headers now supply declaration-owned calling interfaces.
The type bridge reconstructs them from validated descriptor signatures and
canonical import origins, preserving aliases and method identities. Omitted
arguments use an External ParameterDefaultRef containing the canonical owner,
slot and authored revision; defining-program SymbolIds stay in that program.
Generated std query interfaces are consumed by signature lowering as well.

The runtime can prepare an installed std default's actual expression program
and map preceding values through the existing checked revision bridge. Native
method/impl binders, spec Self/arguments and method type/Effect binders are derived
from checked invocation evidence. Results are projected using the declared
parameter type, not the containing callable's return type. The same program
boundary preserves Handler, budget, cancellation and policy state. CPU workers
also select the machine's current program after a program transition.

Raw callable and action metadata now also carry ordered default-slot records.
Each record retains its calling position separately from the canonical expression
reference's defining parameter position. Action value-parameter names are separate
from static selector names and selector defaults. Source projection covers flows,
agent entries, tools, callable specs, spec methods and actions, including bodyless
signatures. Default revisions participate in public metadata fingerprints.

Package schema 56 codecs and graph budgets retain these records. Package readers,
writers and dependency-path conversion preserve the canonical references while
rewriting public lookup paths. Frontend input validation checks declaration names,
slot ordering and owner kinds; the loaded defining program still supplies actual
execution evidence. No default expression becomes a synthetic callable body.

The type importer supplies these interfaces to spec/action signatures as well as
ordinary declarations. Std action collection preserves the imported source
signature. Execution-role default Handler rows keep their checked native rules,
with type and Effect binders mapped to source names. Runtime action ABI checking
uses the complete declared argument binding when values were omitted; supplied
source expressions still retain their original trace coordinates. Default action
environments read the actual perform declaration rather than an arbitrary alias.

This implementation has not been compiled or executed. Ordinary dependency
packages still need execution-program loading, and frozen external revisions,
complete default-aware latent/invocation analysis and CLI/model-tool entry paths
remain unfinished. These changes do not establish verified cross-package default
support or independent std bootstrap.

Implementation descriptor headers now read inherited defaults from the checked
method interface while keeping their authored body parameter names. The matching
Etas ParameterInfo record and ParameterDefaultInfo/Owner data
definitions are written. Residual, frozen and candidate descriptor inputs now
retain their defining source bundles. All of this remains uncompiled and
untested. The installed query view now projects parameter names and canonical
default references from compiled callable headers, keyed by CallableId. Registry
image schema 10 transports this generated table in canonical order without
descriptor-graph indices or caller-local symbols. Source-binding-only and
native-only views clear the source table. This does not add any handwritten std
API declarations or native default implementations.

For source spec implementations, the query view takes public parameter names
from the checked method calling interface. Callable descriptor/body names remain
authored lexical names. This preserves inherited named-call contracts without
renaming the implementation's HIR parameters.

The compiler-data projector now writes ParameterInfo.default as the declared
Etas record and ParameterDefaultOwner enum. It projects DefinitionId, AgentId,
CallableId and GenericOwnerInfo into ordinary checked source records/enums;
ImplementationId retains package, module, subject, selected spec and header
fingerprint instead of collapsing an implementation into a declaration identity.
Record projection supports structural aliases as well as nominal records, and
enum envelopes use the caller's checked payload layout. The explicit legacy
provider retains its earlier metadata-value and ParameterInfo shapes. This is
runtime representation conversion for compiler-owned descriptions; it adds no
Etas API implementation in Rust.

Native output conversion is now written for all existing compiler metadata
payload variants: type descriptions, Effect rows, action summaries, trace
summaries and constraints, determinism, annotation/static values, diagnostic
descriptor details and trace witnesses. Type expansion preserves declaration
and generic identities; a repeated named type becomes a Reference, including
the actual arguments of a recursive application. Function types share the
same expansion context through parameters, defaults, bounds and Effect rows.
Language builtin constructors and latent Task Effects have explicit source
variants rather than fabricated declaration identities.

Trace source data retains callback signatures, observed arguments, Handler
control/results and callable outcomes. Unknown ordering, widening and unresolved
callbacks remain explicit in the returned expression and summary gaps. Inline
constraints retain their alternatives and selector-bound rules without inventing
an owner. Witnesses preserve their complete canonical label path, including
return/non-return edges; a type fingerprint is explicitly distinct from a type
description. Module annotation references and collection kinds are preserved.
This is written output marshaling, not verified compiler/host integration or
completion of the standard library. Raw spec/action default interfaces are now
written but unverified. Complete import binding, external dependency body revision
pinning and cross-program default coverage remain unfinished. None of these paths
has been compiled or exercised in this batch.

These changes have not been compiled or tested. Scope inference, role-table
loading, inherited policies and metadata roundtrips still need consolidated
verification; dynamic/aliased callback values need further integration. Generic
continuation materialization and source task checkpoint execution remain
unverified. This batch does not establish that the migrated task subsystem or
the whole source standard library works.

Remaining collection integration and host-data bridges are unfinished. Rust's
annotation-driven type-role tables have not yet been
removed. Earlier green checks below describe the source tree
before this rewrite and do not accept the current types or a complete std bundle.
The current tree has 122 `.es` files across 51 modules. The frontend artifact
schema is 157 and the library HIR schema is 46; this source batch has not produced
an accepted artifact in either schema.

The current implementation rules are:

- Put a type's method signature and algorithm directly inside its `impl`.
  Delete the former module-level alias; do not retain a forwarding flow.
- Implement ordinary algorithms and policy in Etas. Do not use binding
  annotations, private native aliases, renamed native defaults, empty bodies or
  placeholder failures to count an API as implemented.
- Keep actual execution mechanisms explicit: primitive value representation,
  retained compiler objects, host I/O/storage, execution context, continuation
  transfer, and enforced authority/budget/cancellation boundaries.
- Finish the complete source body batch before consolidated compilation,
  execution tests and fixes. Do not add per-module defensive test matrices.

The ownership rule also covers associated constructors and private type-owned
helpers. An associated flow inside an `impl` may take the owning type as an
ordinary argument: that does not make it a receiver. Source method collection
uses the declaration's `self` marker to distinguish the two forms. A direct
constructor/copy/method/spec call chain and its binary metadata roundtrip pass;
this verifies method dispatch and ownership, not full std delivery.

There are zero binding annotations in the source tree. The frontend annotation
lowering entry has been removed. The bootstrap catalog now contains 53 contracts,
13 native descriptors and 57 type references, after removing 65 ordinary method
contracts, 140 other ordinary flow contracts and four ordinary prelude flow
contracts. Days, LastTurns and SummaryPlusRecent are imported from their actual
source enum namespaces; approve is an ordinary source flow. These APIs have no
handwritten bootstrap flow signature. The two former scheduler transfer
contracts and the legacy start flow contract are also absent; their actual source declarations select verified execution
nodes rather than competing with native method aliases. The excess role/reference contracts still
need review before the full library meets the minimum compiler/host ABI gate.

Before the native type rewrite, the first installed-SDK milestone passed with the complete legacy registry
factory compiled out. The producer compiles the maintained join body and the
Array and Error declarations. Adding one `.es` file supplies a new generic API
whose body constructs a private source type and calls its private impl method.
An application in another process imports and executes the new API. Two producer
processes generate identical manifests and artifact bytes. The CLI installs the
binary, sources and artifact together; after moving that prefix and deleting the
original source directory, the installed binary discovers its SDK and checks and
runs the application outside the repository. Rebuilding the installed sources
after changing join changes the application result from 42 to 43.

The installed query registry retains only source-verified role and primitive
bindings and their native type-reference closure, preserving stable IDs. It
publishes ordinary types, effects, specs and callables from checked bundle
metadata. An unpublished bootstrap API is unavailable. Effect inference registers
core tags only when the selected registry contains the corresponding effect, so
a small SDK does not acquire unrelated standard effects. Resolved-reference
validation remains enabled. The archived producer registry still describes its
checked compilation environment; it is not the consumer's public API catalog.

The permanent CLI case is
`source_sdk_join_and_added_generic_api_run_after_install_and_relocation` in
`etas-main-integration/crates/etas_cli/tests/cli/std_source_sdk.rs`. Its passing
processes used a CLI built with `--features forbid-legacy-std`; a separate probe
confirmed that the explicit legacy entry panics because its factory is disabled.
This accepts the first focused SDK milestone, not a full 121-file std bundle or
the protected I/O, handler, task, IDE, compatibility and release gates.

The source now includes collection algorithms, prompt encoding, JSON and HTTP
codecs, SHA-256, decimal integer/float conversion, Unicode case mapping,
normalization and grapheme traversal, task composition and Handler policy,
conversation protocol assembly, Store paging/clear/write policies, filesystem
locator handling and ordering, and compiler body formatting. Source formatters
use retained raw tokens; the public `SyntaxBlock.text` projection must use the
source formatter rather than the old Rust formatter.

Command configuration belongs to `impl Command`; handle operations belong to
`impl CommandHandle`. For example, construction is `Command.new(program, args)`
and configuration is `command.with_env(environment)`. The old module helpers
are removed rather than forwarding. The direct host-operation flows `run`,
`start` and `reconnect` remain module APIs; `std.command.run` is a public import
of the canonical host command flow.

The runtime has twelve execution roles, including Scope.enter, Task.start and
the private completion/cancellation mechanisms. Bodyless execution
mechanisms remain separate from the source type rewrite; their bindings must be
adapted to the actual source wrappers, not used to restore ordinary native APIs.

Before the type rewrite, the source-policy runtime batch passed. Scope.enter
allocates a fresh owned context and drains its children on exit; the caller
installs the source Handler. Both the legacy entry and this source primitive use
the same physical context mechanism. The source entry does not call a native
public run implementation or install a hidden native Handler.

The driver invokes the checked source Scheduler.select implementation for each
runnable snapshot. Snapshots are sorted by the owning run's enqueue sequence;
new, yielded and woken work receive new sequence numbers. Policy state has value
semantics and updates through the existing exclusive receiver lease. Selection
requires actual checked deterministic, action-free and non-suspending behavior.
Invalid indices fail rather than selecting another entry. Admission, resource
permits, frames, waits and continuations remain runtime mechanisms.

The pre-rewrite execution case used the maintained Scope.enter declaration,
Handler adapter, transfer declarations and FifoScheduler body. FIFO produced
checkpoints 20 then 22; a source policy choosing the last ready entry produced
22 then 20. Both returned 42. A separate yielding case also returned 42. All six
JSON checkpoints resumed to 42, and subsequent saved policy states matched their
uninterrupted states. The supplied policy value stayed unchanged. The same case
passed with the complete legacy registry factory disabled, using the minimum
Task/start ABI and no native run contract. The prior source iteration case still
returned 1174 and resumed all sixteen checkpoints.

Those checks do not establish current role-free type execution, the public run
default/lifetime contract, source task start/completion/cancellation, installed
Handler ownership, or a full task SDK. The current source combinators request
policy inheritance, but this written path has not been verified. The current type
rewrite supersedes the earlier Task record/opaque-role mismatch instead of
weakening that old check. No whole-library acceptance is claimed.

Memory selection owns query/scan policy, pagination, IEEE vector conversion and
key/value/version prompt encoding in source. Store preparation owns condition
parsing, operation key spelling, canonical tagged encoding and BLAKE3 hashing.
Session context operations now belong to `impl SessionConfig`, including
lossless fence parsing and exact publication fingerprint binding. The backend
still authenticates evidence and checks the context CAS atomically.

The first consolidated check exposed incorrect record initializer punctuation;
initializers now use `=`, while record type fields retain `:`. Match-arm
statements and character receiver declarations were corrected as a batch.
Primitive inherent impls now resolve to their canonical `std.core` type.
Bodyless flow declarations have explicit AST/HIR variants and no synthetic
executable block. `self` and `var self` remain distinct in serialized HIR and
typed method contracts. Updating execution now requires a checked direct local
`var` or the current method's updating receiver and a transitive non-suspension
proof. Bodyless flows cannot publish without a verified execution contract.
Before the native type rewrite, the frontend artifact schema was 141 and the
library HIR image schema was 31. The current compiler role execution ABI is
revision 20. Checkpoint format v89 retains
source iterator calls and argument evaluation before acquiring an updating
receiver, captured Handler environments, and source policy state plus enqueue
sequences. A held receiver update cannot be checkpointed.

Updating receiver mode now survives typed method facts, behavioral-spec
declarations, public method metadata, package adapters, and library HIR images.
Source and imported spec implementations compare that mode as part of their
contract. The mode remains separate from overload selection, and an updating
method cannot become a bound method value through the ordinary value-receiver
path. Package metadata container schema 53 preserves the new receiver ABI and
spec flag; method fingerprints include updating mode.

An abstract `var self` spec signature can now be checked and exported without
claiming that an executable update has been verified. Source spec projection,
binary roundtrips and two independent library sessions preserve the mode.
Executable methods with the required proof now evaluate ordinary arguments
before acquiring access and reading the receiver's current value. Frame aliases
share the exclusive binding check. The callee publishes its final local `self`;
the caller commits only after normal completion. Escaping errors and unwind
discard the update. A task cannot yield or enter a host boundary while access
is held, and cancellation/fault releases its active leases. Returning `Err` is
normal completion and commits.

The cross-program call path now projects the updated receiver using its input
ABI, separately from the ordinary return value, before committing. Independent
installed/imported updating execution still needs acceptance; a local execution
case does not establish that gate. Mode-only overload rejection and broader
spec/handler execution proofs also remain to be completed or verified.

A separate transitive suspension analysis now records known suspension and
unknown execution behavior independently of Effect rows and determinism.
It follows checked source calls through recursive components, distinguishes
closure creation from invocation, and treats missing callback/import/primitive
evidence as unknown. Source `Iter.next`, unresolved spec dispatch, pipelines,
frozen bodies without a current proof, and handler-specific error/resumption
behavior still require further discharge. The fixed non-resumable Error ABI
can unwind an update without holding it across an outer handler; primitive
integer indexing of primitive storage follows the same error path. Explicit
handler scopes remain unknown until their behavior is proved.

Suspension facts survive body artifact projection and std bundle restoration.
Checked declaration headers also carry optional suspension analysis in
descriptor wire schema 16; absence preserves unknown behavior. Canonical
callable identities select imported declaration/method contracts, and the
portable descriptor fingerprint includes the new facts. Producer binary
metadata transport is verified; a separate installed-package consumer check
is still required before claiming complete imported updating-call support.

The consolidated syntax audit covers all 121 files. List empty/cons patterns
now pass through parsing, HIR, type constraints, exhaustiveness, alias/capture
analysis, serialization and runtime matching. Matching shares immutable tails.
A production CLI example executes ordinary and generic methods directly from
`impl List` bodies and returns 617 after a shared-tail update, preserving the
original list's value. This verifies the language mechanism using the legacy
provider; it is not standard-library artifact execution acceptance.

The compiler/runtime now checks and executes readonly numeric/char `.bits`,
string `.utf8` and byte/count `.length` projections on actual primitive and
structural collection types. Plain scalar constructors consume bit patterns,
character arrays or byte arrays directly, with width truncation, signed
two-complement representation and IEEE float bit reinterpretation. They do not
make arbitrary nominal records into primitives or expose protected construction.
A production CLI program returns 1111, including global constant projections,
Unicode, float/signed bit round trips and collection lengths. Field references
also retain the actual global initializer in callable analysis reachability.
This runs with the legacy provider and is not full std artifact acceptance.

The source `Message<T>` role now keeps its declared nominal record instead of
being replaced by the historical runtime Message type. Source type references
and native host signatures in a source session share that checked constructor.
Legacy sessions retain their existing Message representation: a CLI program
creates a legacy message, reads its content and returns 42, with the expected
message-created event.

The canonical library case copies the maintained eight-field Message
declaration. Its generic type method reads and copies the envelope fields into
a message with a different payload type. Two independent source sessions yield
identical bundle bytes, and the restored call retains its non-suspension proof.
This removes all eight Message field representation errors in the whole-library
check. Source context capture/recording, payload conversion, provenance ABI,
host/session/AIR boundaries and installed message execution remain required.
The existing iterator/numeric/updating/filesystem execution case returns 1166 with
fourteen successful JSON checkpoint resumptions; that regression is not evidence
of source Message runtime acceptance.

`WorkspacePath<R>` is now the source-declared private relative-locator record.
The compiler verifies its one private string field and exact Region bound,
and binds runtime roles to actual source constructors instead of bootstrap
parameter placeholders. No path value contains a host root or grants access.

All five raw Fs actions now invoke the physical host filesystem after language
handlers and residual policy: read, write, list, stat and atomic_replace. Their
checked selectors and path regions agree; authority, trace and budget come from
the caller. Bounded reads and listings decode into the checked source Result and
limit-error enums. Directory entries become source-owned WorkspacePath records,
with their region, parent and limits checked at the host boundary. Stat decodes
through the actual checked source type. Public sorting and error policy remain
in Etas, with no native std.fs API callable in the minimum bootstrap.

The minimum Fs action ABI matches the source limit/content payloads. Source
IOError is a nominal string carrier whose exact primitive representation is
checked. Other host imports remain opaque unless their ABI explicitly gives a
primitive representation. Native registry image schema 8 records that ABI.

The consolidated case loads nine maintained filesystem flows, including
read_bytes, read_bytes_limited, write_bytes, list, list_limited, stat and
atomic_replace, against LocalFilesystemClient with a real workspace sandbox.
The actual source Order, AVL OrderedMap, Iterable and List.reverse implementations
perform directory ordering. It also inserts three keys and invokes the source
map's get and contains_key methods, including its stored comparison function.
The case returns 1166; all fourteen JSON checkpoints resume to that result,
including a checkpoint holding source path/stat values. Its fifteen initial
requests retain the exact region, caller authority/sandbox, trace and budget
limits. Caller Handlers catch source-raised read and directory limit errors.
A test-only method inside the path impl constructs the fixture locator; the full
source path-normalization algorithm has not yet been accepted.

The canonical case checks these maintained bodies in two independent source
sessions and obtains identical bundle bytes. Specs are collected before impls
while preserving the original implementation groups, so an impl can appear in a
file ordered before its spec. Callable analysis retains generic data origins
without requiring a statically noncallable subtree to supply an invocation
contract; checked callback arrows must still agree. In source-bootstrap sessions,
ordinary source types own their canonical descriptive identities even when a
transitional compiler query has the same name. This does not grant a runtime role.

Both Error<T>.raise and Error.raise<T> resolve through checked action identity.
Selector arguments normalize to the typed Error effect; the VM reads the checked
invocation rather than identifying errors by a source name. Permission,
cancellation, I/O-error and budget acceptance across the complete filesystem
scope, full source path normalization, and independent installed execution
remain required.

Genuine execution declarations for abort, checkpoint, Data.capture and execution
context observations now lower to explicit HIR Runtime nodes. The Etas declaration
owns the public signature; type finalization verifies the minimum execution ABI
before Effect analysis. Capture validates its actual source enum and record
layouts directly, without adding a duplicate native API to the registry. These
nodes have no invented source block or forwarding call. The VM executes each
mechanism directly. Ordinary library bodies cannot
acquire these roles: source-bootstrap mode, canonical ownership, visibility and
declaration shape are checked.

Abort has a checked non-return outcome and cannot suspend. Checkpoint requires
interpreter checkpoint orchestration and is explicitly suspending, independently
of its empty escaping Effect row. Annotation argument bodies remain indexed for
metadata ownership but cannot become a declaration's primary execution body.
This fixes the incorrect analysis of a role annotation string as a callable body.

The consolidated case now loads the actual abort/checkpoint declarations and
assert body. It calls checkpoint both directly and through a typed function
value, restores sixteen JSON checkpoints to 1174, and executes source
assert(false) to the expected ExecutionAborted fault. The signed-minimum numeric
fault case remains covered separately. Two source sessions produce identical
canonical library bytes, and restored declarations preserve their Runtime roles,
source archives, suspension and orchestration requirements; all ordinary source
callables still require actual AST bodies.

Data is an ordinary Etas enum. Its capture method stays inside impl Data and
uses the explicit VM representation role. The parser retains method annotations,
HIR indexes their real owner, and checked method targets distinguish an actual
AST body from a verified execution node. The runtime never invents a body or
forwards the method through a module flow.

Capture traverses ordinary nested values and emits the checked source Data
constructors. Existing Data is validated and returned unchanged. Protected
wrappers, execution values and authority-bearing source types are rejected at
any visited nesting level. Incremental enforcement reuses the caller's real
StorageLimits accounting before constructing the captured result; cancellation
and wall-time exhaustion retain their execution-control semantics. The combined
runtime case covers nested tuples/records/options/JSON, Data map identity,
nested workspace-path/callable rejection and capacity limits, including source
Data values held across JSON checkpoints. Source tagged/JSON codecs and the
complete Prompt/storage paths still require whole-library acceptance.

StorageContext is an ordinary source record with private fields. Its current
method returns the caller's actual storage limits and Unix time; nonce obtains
only OS entropy. The private StreamClock.now supplies Unix milliseconds to the
source timeout algorithm. These associated execution declarations validate
canonical owner identity, visibility, private snapshot layout and result types
before Effect analysis. They add no public native registry declarations.

All three observations have non-deterministic summaries, including independent
contributions propagated through source callers. They are physically immediate
and require no external adapter. An empty escaping Effect row therefore does
not admit these operations to deterministic CPU workers. Cancellation and the
actual execution wall-time budget remain enforced at the mechanism boundary.

The consolidated runtime case uses non-default storage limits and checks all
eight snapshot fields. The actual Etas StorageContext.operation_key and private
u128.hex_fixed bodies execute with actual text scalar traversal, byte get and
Index implementations. The returned key is accepted by StorageOperationKey.parse
and has the configured 7200-second retention. A clock value also survives JSON
checkpoint/resume. Complete stream reduction, memory transport/write policies,
independent installed execution and process replay still need acceptance.

The focused filesystem frontend fixture now loads the actual ordered collection
dependencies and validates performed source actions and their checked payloads.
IOError retains its actual source nominal string representation. No source Fs
flow is treated as a native intrinsic default. Full-library validators and all
other module ABI requirements remain in force.

A value or reflection whose spelling is both a child module and a public value
export resolves to that checked module's parent export. This allows the ordinary
std.runtime.checkpoint value reference to select its source declaration. Source
callable aliases also retain source effect and VM targets when a transitional
native query shares the name. Installed source-provider and cross-owner primitive
execution still need acceptance.

The source-owned `Iter` role now has a checked `next` callable field. The VM
invokes that actual callable through its existing call machinery and saves the
cursor in its continuation. Byte traversal uses immutable primitive storage.
A consolidated source-bootstrap runtime case uses the maintained Option and
Iter declarations and the actual `impl Iter.map` body, composes i32-to-u128 and
u128-to-u128 maps, reuses the original iterator, and exercises termination,
continue, break and byte traversal. The case also uses checked indexing and
captures its actual `IndexError`, constructs `Money` in source, and invokes the
maintained `impl Range.closed/open` bodies. It also loads the maintained Result
declaration and exercises receiver snapshots, nested updates, generic updates,
argument-side updates, normal `Err` commit, and nested update rollback on an
escaping Error. It returns 1166, including the maintained math, collection and filesystem algorithms; all fourteen
JSON-encoded checkpoints resume to the same result, including suspension inside `next`, before receiver access and with source path/stat values.
This is source-bootstrap execution, not installed artifact-provider acceptance.

`Range` now shares the compiler range value used by range literals. Its source
declaration specifies the two endpoints and inclusion flags; the compiler
checks that layout, and the runtime only constructs/projects that value.
Algorithms remain in the owning impls. Structural alias impls inherit the
actual declaration's generic bounds, including `Range<I ~ Index>`, and retain
their canonical role when exported and restored. Imported enum constructor
patterns resolve checked source declarations before consulting the selected
provider, fixing false unreachable arms and generic conflicts in HTTP decoding.
The bodyless `Array`, `List`, `Map`, `Set` and `Slice` role declarations also map
to their structural compiler value families. The consolidated execution and
bundle cases load those declarations from the maintained source files.

`break` and `continue` have `never` result types within value branches. While
loop completion still needs integration: the current runtime returns normally
at its iteration cap, whereas design section 4 in
`04-flows-human-gates-and-protocols.md` requires a budget error when the condition
remains true. Bounded `while true` is therefore not marked `never` merely from
its condition. The remaining source loop fallthrough diagnostics are retained.

The source-owned, empty `std.math.Number` spec now names the compiler's closed
primitive numeric family. Explicit source impls cannot add members. Generic
arithmetic and ordering require checked bound evidence after type solving;
unconstrained parameters do not acquire numeric operations. The declared bound
and actual maintained `abs/min/max` bodies survive independent source sessions
and canonical library HIR round trips.

`abs/min/max` signatures and algorithm bodies live in `math/complete.es`.
Readonly numeric representation projections provide `magnitude` (same type),
`is_nan` (bool) and `bits` (u128, also available under a checked Number bound).
These are execution rules, not native std API defaults. Magnitude preserves
integer type/width, leaves unsigned values intact and clears the floating sign
bit without rounding or canonicalizing a NaN payload. Signed minimum follows
the existing checked arithmetic-overflow fault. Etas branches select min/max
operands, including left NaN payloads and the specified signed-zero ordering.
The combined source execution case covers wide u128 ordering, f32/f64 NaN
encoding, both signed-zero orders, generic addition, a constant magnitude
projection, and the required signed-minimum overflow. This is checked source
and library HIR acceptance; independent installed-provider numeric execution
remains to be discharged with full std delivery. Two independent CLI producer
processes previously generated the same focused numeric SDK manifest and artifact
hash. The unrelated AssistantTurn bootstrap reference that blocked that consumer
has now been removed. The new join/generic SDK case above passes, while installed
execution of the complete numeric source subset still needs its own acceptance.
Representation output primitives are now materialized before field solving,
including in a minimal library that never explicitly names `u128`.

Checked callable fields retain their parameter origins and concrete action
behavior. Sixteen focused existing callback checks pass, including handled
actions and trace multiplicity. Method application selection and lambda
validation now preserve canonical impl-owner bindings alongside the method's
own generic arguments. Source Option/Result constructor binders and public
source re-exports also take precedence over bootstrap placeholder spellings.
Independent source sessions produce identical bundle bytes with two different
owner instantiations of the same method. Public latent contracts, method ABI
materialization and representation roots use stable checked-identity order.
Whole-library Effect/provenance and implicit source-iteration analysis remain
part of the outstanding full-library acceptance work.

Behavioral members of role-bound specs now use ordinary source spec checking.
The `Index` signature, integer implementation, generic call and primitive method
owners survive canonical bundle round trips. Fully qualified source calls use
public checked module exports; they no longer require a handwritten flow
signature in the bootstrap catalog. The selected std provider still supplies
its own other exports to source overlays. A legacy-provider CLI consumer using
qualified native and source text calls returns 42.

The syntax diagnostic count is now 15, down from 130 at the previous checkpoint.
Remaining syntax errors concern two defaulted parameters and two compiler
constant declarations. `Logical` is a source `TraceLabel` value; its erroneous
`TraceSpec` declaration was removed. Formatter methods now update local buffer
copies and write them back instead of updating field receivers.

The 64 existing HIR checks and three focused source-library checks pass: adding an ordinary source API,
checking an unreferenced private body, and canonical bundle round trips through
two independent sessions including a generic method with List patterns. The
latter exposed a constructor binder bug: the bootstrap ABI's placeholder names
were ignored in favor of hard-coded T/E. Constructor lowering now uses the
actual declared generic positions.

Imports are file-local. Explicit enum imports were added to five HTTP codec
algorithm files and four text algorithm files; public re-exports in a sibling
file do not create local bindings. The last pre-rewrite whole-library check had 224
front-end diagnostics and no accepted bundle. A source build with declaration
or type errors stops before downstream effect/provenance analysis, preserves
the original diagnostics, and still performs complete analysis on valid input.
The check covers all 121 files; downstream effect/provenance analysis remains
required once declaration/type errors are fixed. No valid-library performance
acceptance is claimed.

The pre-rewrite Rust CLI built successfully with local workspace patches. The
current native type definitions have not been compiled. The
runtime lib-test target remains unaccepted: the last broad check encountered
stale model-loop/message/checkpoint fixtures and Handler literal changes. They
have not been suppressed or replaced with compatibility stubs. The focused
source execution target compiles and passes. Standard-library
artifact execution now passes for the focused join/generic SDK case above;
full runtime acceptance remains pending. The older whole-std SDK integration case
also contains stale binding-based fixtures and remains unaccepted; passing the
new focused case does not establish that the full CLI SDK suite passes.

Beyond the accepted path/stat/collection cases above, protected source data
shapes, private execution projections, existential command directories, other
host action payloads and conditional spec witnesses still require integration. Source
host actions must retain the exact region/store/session/stream authority and
caller budget, trace and cancellation context. They must not widen to wildcard
or root authority to make a build pass. A private source clock keeps one overall
stream deadline across reads and resume.

The machine-readable ownership and outstanding declaration inventory is in
`etas-main-integration/docs/migration/std-type-method-migration.json`. Historical
667-symbol declaration inventories are migration baselines, not current API
implementation coverage or execution proof. The previously unspecified seven
flows now have source bodies based on the architect's supplied semantics.

Std 0.4.0 is the reserved, unreleased minor version for this source feature.
Legacy remains the release default. All five integration PRs remain drafts at
their earlier published heads; this implementation batch is uncommitted and
unpublished. Whole-library bootstrap, bundles, runtime execution,
checkpoint/resume, full-library external offline installation, IDE, compatibility
and performance/release gates remain required before delivery.

The native declaration inventory is
`etas-main-integration/docs/migration/std-native-type-definitions.json`. It lists
all 88 removed bindings and the fourteen private execution capabilities, and
marks compiler/runtime integration and testing as pending.

## Current correction: array storage and record visibility

The user explicitly limited recursive definitions to appropriate structures.
The source Array tree implementation has been removed; the underlying array
value remains Rc<Vec<InterpValue>> with copy-on-write value semantics. No public
Array type-role binding or native std API wrapper was restored. Record fields
also now follow the design's public default; explicit private fields keep their
specified visibility. The current correction is written but uncompiled/untested.

### Typed callback parameter projections

Checked callback parameters now retain a typed root and a structural path through
heterogeneous tuple bindings, generic calls and returned closures. Package metadata
schema 60 carries scoped parameter paths and standalone request projections; path
ordinals are canonicalized before publication. Frozen callable environments retain
and relocate these paths against the restored checked parameters.

The existing 16 callable application cases pass, including different tuple result
types and captured callbacks, and the CLI compiles with `forbid-legacy-std`. The existing driver
replay case also passes: a fresh metadata-only consumer checks both heterogeneous
callback results and a returned closure, retaining all four stdout request events
without dependency source reanalysis. This is compilation evidence, not runtime
execution acceptance. The strict release build also passes. The new cache-off
whole-library run reports 88 diagnostics (82 unique), resolving all nine Task tuple
callback errors with no additions. It took 233.85 seconds, including 225.58 seconds
in memory provenance analysis. General aggregate provenance, cursor callable
environments, captured Error effects, source default Handler associations and
dynamic limits remain incomplete. The full SDK and the original release
acceptance gates are not passed.

### Callback fields and non-suspension proof bindings

Checked record fields and partially resolved value paths now keep the selected
parameter path, including fields referenced inside nested closures. Capture
bindings retain those selections while the complete argument is available. A
single stored field can preserve its payload origins without merging sibling
fields; concrete selected data types with no callable or generic payload prove
an empty origin set. A function's empty Effect row does not supply this proof.

Native call argument facts now use parameter targets rather than bare symbols.
The non-suspension analysis resolves the same selected targets through actual
arguments and captured environments. Frontend schema 174 and library HIR schema
62 preserve this change. The parser also distinguishes a named anonymous record
field from a lambda's arrow in its value before considering map syntax.

The existing 16 frontend cases pass with body-fact CBOR replay. The existing
metadata-only driver consumer case also passes: heterogeneous record callbacks
and a returned closure retain six stdout request events across the complete
producer/codec/driver/import chain, without reanalyzing dependency source. These
are compilation facts, not runtime execution acceptance. A current whole std
check is pending. The previous completed diagnostic receipt remains 88/82 errors;
no full std SDK is accepted. Invocation-return cursor origins and the remaining
Error/default Handler/dynamic-limit work remain open.

The first full field check exposed sparse callback records: ordinary data
siblings had prevented recovering a comparison callback from a local collection
value. A selection can now preserve the carrier's origins only when all other
checked fields exclude callable and generic payloads. Two callback fields remain
distinct; callable arity is not a selection rule. The existing positive source
case now invokes a comparison field through a record containing ordinary data
and proves the actual concrete caller non-suspending. All 16 cases pass; full
library verification of this correction is pending.
