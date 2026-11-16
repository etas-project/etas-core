# Complete standard-library declaration migration

All public paths in the 667-symbol legacy inventory now have corresponding
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
