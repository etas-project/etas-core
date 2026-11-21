<!-- Historical recovery snapshot, archived 2026-11-21. Not a current specification, implementation status, or engineering task assignment. The current language SPEC and main implementation take precedence. -->

# Reflection In Package Metadata

Status: Target contract for proposed SPEC chapter 18.
Owner: Architect
Updated: 2026-11-03

## Responsibility

etas_package_metadata transports compiler-produced public definitions and
optional retained code. It depends on neutral etas_code contracts, not AST/HIR,
runtime values or frontend compilation.

Current PublicMetadata already carries AnnotationMetadata. Reuse that producer/
consumer path and preserve structured annotation values; do not add a second
unrelated annotation registry. Definition/revision identity and retained-body
availability are additional contracts.

## Layered Structure

Extend the current model/proto/container organization:

```text
src/
  model.rs                    # package/public metadata composition
  code/
    mod.rs
    definition.rs             # canonical callable/agent/method definitions
    body.rs                   # optional retained body envelope and access status
    dependency.rs             # frozen binding/environment/dependency identities
    validation.rs             # graph/reference consistency, no language checking
  proto/
    schema.rs                 # versioned DTO graph/table representation
    codec.rs                  # typed round trip, no inferred source semantics
    budget.rs                 # bounded graph, bytes, nodes and depth
  container/
    mod.rs                    # section framing, integrity and compatibility
```

Semantic checking and mapping between local compiler IDs and stable definition/
type/spec IDs belong to frontend/package_metadata and frontend/code/artifact.

## Definition Contract

A definition record identifies canonical package/module/declaration/kind and
method/spec implementation ownership. Ordinary-flow records preserve the
checked callable interface F, generic/effect binders, revision and frozen
behavior-contract reference separately from the erased CallableDef view.
SignatureInfo is descriptive, not a way to reconstruct typed evidence. The
primary agent invocation and its
lowered default method reference one body/summary identity. Additional methods
have separate records; never publish them as one homogeneous callable type.

Preserve signature binders and parameter names, original versus inferred type
information, annotations, tool/spec references, dependency fingerprints,
declared escaping bound, inferred escaping/requested/default-handled rows,
determinism and static trace-summary references. Plain descriptors remain
descriptions, not checked-artifact or spec-satisfaction evidence.

Type/row/trace graphs use explicit node references and canonical structured
encoding. A display path, source-local TypeId or unqualified method name is not
a portable identity.

For public helpers returning a dynamically compiled callable, preserve the
validated result-to-descriptor contract-origin relationship and its type/row
substitution templates. A parameterized origin remains symbolic until the
consumer substitutes the checked descriptor argument. Branches retain all
possible origins. Do not serialize a compile result as the original body target
or synthesize its behavior from F alone. Freeze/interface identities and actual
revision summaries are distinct; missing required provenance is incomplete
metadata, not an empty-effect declaration. The producer/consumer use the existing
callable-return/latent metadata path rather than a second reflection-only ABI.

### Portable Method Declarations And Evidence

Follow the [method-resolution contract](../../../etas-frontend/docs/architect/method-resolution-design.md#8-package-and-diagnostics).
Public inherent/associated methods and behavioral spec requirements/impl members
must preserve canonical owner/callable/implementation identity, receiver mode,
generic binders, checked signature, visibility, body/executable references and
solved behavior contracts. A spec membership tuple alone cannot supply a method
signature or implementation target. Do not infer a method table from its name,
record shape or a matching representation type.

Frontend exports/validates these facts and builds its shared MethodIndex;
package manager only transports the compiler-produced schema. Alias/re-export
identity and source availability remain distinct. Version incompatible method
sections/cache schemas together and reject missing/conflicting executable facts.
Publishing a method signature is not implicit export of private source. Direct
method reflection remains the SPEC's erased/read-only view; this protocol does
not make methods standalone editable/compilable flows or create new authority.

### Residual Obligations And Plans

The shared wire model is defined in
[Residual Checks And Runtime Trace Monitors](etas-residual-checks-design.md).
Export required typed obligation templates, normalized plan references, event/
scope/predicate bindings, generic substitutions, exact coverage and semantic
versions alongside effect/trace contracts. This applies to ordinary public
callables as well as reflected targets; a list of check-description strings is
not a portable executable contract.

Source locations use the independent SourceRevisionKey, not the final checked
RevisionKey. Preserve source/obligation identities and semantic plan hashes
through projection/re-export, with the checked revision/contract linkage stored
in a separate BoundResidualPlan record. Follow the
[acyclic identity and binding protocol](etas-residual-checks-design.md#31-acyclic-source-and-obligation-identity):
neither the current package output digest nor the final bound wrapper can be an
input to the semantic identity whose metadata is being produced. Known external
dependency fingerprints remain valid input dependencies. Validate all binding
relationships on consumption; do not renumber IDs or infer missing links from
matching source text/display names. Bump incompatible identity/binding schemas.

Preserve complete OR alternatives, before/after correlation and exit semantics;
never flatten plans to requested-action sets or display labels. Relocate local
sites/types/resources to canonical identities and validate references at the
frontend environment boundary. Consumers revalidate target/guard compatibility
before preparation. Checksums or producer claims of installation cannot grant
current authority or replace engine-native binding evidence.

Do not serialize live automata, callbacks, Host grants or mutable invocation
state into public metadata. Engine checkpoint state uses its own versioned
validated codec. Version incompatible residual sections explicitly; a legacy
string check cannot silently become an empty plan. Metadata-only inspection may
honestly expose unavailable support, but missing required plans block execution.

## Optional Bodies And Visibility

Body availability is independent from a public signature. A bodyless or hidden
payload is represented as unavailable and projects to body=None; never encode an
empty block to mean unavailable. Bare agents have no default signature/body;
ordinary flows and method descriptors always have signatures. These cases must
not be collapsed into a fake unit -> unit callable or an empty implementation.

Use the owning manifest's [reflection].retain_bodies and export_bodies, both
empty by default. Entries resolve to fully qualified owned items, not file paths;
export requires a public item and implies retention. Apply equally to agents
and flows; runtime revisions inherit the target policy. Consumers cannot export
a dependency's hidden source through their own configuration or re-exports.
The neutral envelope names the syntax schema, immutable source revision,
fingerprints, binding references, payload size and access policy. Frontend owns
native syntax decoding/revalidation; Core must not grow another AST parser.

Private dependencies are not promoted to public source by a re-export. Record
the permitted frozen lexical binding environment without embedding live locals,
secret values, connections or producer authority. Package materialization must
not silently fetch hidden bodies to satisfy a pure reflection query.

## Validation And Versioning

Add a versioned optional definition/body section to the existing binary
container. Update section tags/schema versions/codecs together; reject
incompatible executable-artifact versions explicitly. Follow existing public
metadata compatibility rules for supported metadata-only producers, preserving
an honest body-unavailable result.

Validate section/aggregate decompressed sizes, node/depth counts, references,
duplicate/conflicting identities, dependency/config/contract fingerprints,
recursive descriptor cycles and body/source mapping before expensive allocation.
Round-trip validation must include the consumer frontend environment boundary.
Residual sections additionally bound predicate/monitor nodes, alternatives,
registers, pending-obligation templates and transition work before allocation.
Reject dangling/cyclic-invalid bindings, duplicate/conflicting IDs, incomplete
coverage and unsupported semantic versions; do not trust a producer's boolean
`verified` flag. Cache fingerprints include the plan and selected target support.

A matching digest establishes bytes, not permission to execute or an authentic
compatibility proof. Binary metadata transport is not a public binary code-load
API: there is no std.compiler.load/Code.load in this SPEC. Public persistence
saves source plus expected canonical target and locked contract/dependency/config
inputs, then restores through typed reflection, parse_body, with_body and compile.
The surrounding declarations must remain available and current authority/limits
apply. Stored summaries and erased SignatureInfo cannot establish a runnable F.
Internal code artifacts still pass the compiler/engine validation boundary in
[etas-code-design](etas-code-design.md).

## Producer And Consumer Tasks

- Frontend emits the neutral metadata from actual checked facts and approved
  retained syntax, with local-to-stable relocation.
- Package manager stores/transports/checksums it, without generating summaries,
  definitions or default handlers.
- Consumer frontend validates, resolves canonical references and exposes only
  accessible retained data. Metadata-only reflection preserves signature and
  summaries while body=None.
- Engine publication requires a real checked candidate; public metadata alone
  is never a runnable agent or flow. Typed descriptor projection to erased
  metadata has no inverse unchecked decoding path.
- Tests cover retained and non-retained packages, aliases/re-exports,
  spec-selected method IDs, generic rows, malformed graphs, forged summaries,
  visibility leaks, stale dependencies, typed flow contracts and explicit locked
  source restoration without runtime-signature casts.
