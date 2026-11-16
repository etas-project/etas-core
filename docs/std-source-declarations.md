# Complete standard-library declaration migration

Public paths in the 667-symbol legacy inventory have corresponding
Etas declarations, constructors or re-exports. Existing source algorithms remain
in place. The additional files are `crates/etas_std/library/std/**/complete.es`;
`std.effects` also completes the previously bodyless native effect declarations.
This is a declaration-writing checkpoint, not a successful build or API parity
claim. Compilation and tests are deliberately consolidated after this batch.

The declarations preserve legacy generic bounds, Effect rows, overloads,
constructors, projected fields, methods, marker impls and prelude names. Existing
native callables use explicit `@std.binding` declarations. Algorithms and policy
still backed by native bindings remain separate migration work.

The consolidated compiler work includes these proposed declaration forms:

- A value selector is written after the action name as `name: Kind`, for example
  `action<R> read<Place: MemoryPlace>(region: MemoryRegion<R>) -> unit;`.
  `MemoryPlace` is the existing selector category. `string` denotes the existing
  string-pattern selector category; resource types denote static resource paths.
  These are declaration proposals, not newly accepted general language syntax.
- A compiler-owned constant has a typed, bodyless `public let` declaration with
  a trusted `@std.role`, preserving the existing runtime constant identity.
- Overloads keep their public name and each signature. Binding selection must
  use the selected signature, including the Code actions and task.join forms.

Seven legacy flow declarations have neither a body nor a native implementation:
math.abs/min/max, agent.group.round_robin, runtime.trace.VirtualStages, and
security.declassify.sanitize/declassify. Their declarations remain bodyless;
this batch does not invent behavior or publish placeholder implementations.

Std 0.4.0 reserves the minor increment for this full declaration feature. Release
selection remains legacy until the complete source/bundle/execution gates pass.


## Latest spec synchronization

The source declarations now track design revision
`etas-project/etas@e69e257ae3fbaed628e85a36eb410c189053a5eb`
(`fix/checked-adt-fast-paths`, 2026-11-16), including chapter 19. The main
branch does not contain this spec revision yet. The source tree has 79 files.

Added declarations cover text length/containment, stderr output without a
newline, the command entrypoint, separate scheduler policy/handle interfaces,
the standard Async handler adapter, transparent JSON, single-step Agent
protocol records/ADTs, checked Tool identities/invocation, and conversation
assembly. Conversation.session is optional for an in-memory conversation.
Compiler.ToolRef re-exports the transparent Agent.ToolRef identity.

The existing `codex/agent-step-v1` Core/frontend/runtime implementations are
being integrated rather than assigning duplicate primitive identities. The
legacy registry remains the release default. Source action signature generics
retain the prefix syntax from the accepted architect proposal; chapter 19's
post-name example has not been used to turn inference generics into selectors.

The compiler/runtime Rust build passes with local workspace patches. The whole
std source build still fails. Default parameters, updating receivers, native
constants/prototypes, overload publication and remaining binding ABI alignment
are pending; the SchedulerHandle alias is transitional and policy selection is
not yet wired into the runtime. This checkpoint does not assert API execution,
minimum-bootstrap acceptance or release readiness. No tests were added or run
for this spec synchronization; unified validation follows definition writing.
