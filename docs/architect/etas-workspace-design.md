# Workspace Binding And Isolation Design

Status: `Architecture accepted; implementation acceptance pending`

Owner: `Architect`

Last updated: `2026-09-10`

## 1. Contract And Language Boundary

This is the shared `etas_host` contract for workspace bindings, filesystem
operations and command isolation, tracked by [core#13](https://github.com/etas-project/etas-core/issues/13).
It refines [Host Design](etas-host-design.md#10-sandbox-and-workspace) without
adding a crate, language keyword or source-level `Capability`.

The [substrate SPEC](../../../etas/docs/design/03-agents-tools-prompts-memory.md)
defines `WorkspacePath<R>` and `std.fs` operations. The
[effect SPEC](../../../etas/docs/design/06-effect-system-and-inference.md)
defines region-indexed actions and spec constraints such as `Within<Parent>`.
For example, the existing signature is:

```etas
std.fs.read_bytes<R ~ Region>(path: WorkspacePath<R>)
    -> bytes ![Fs.read<R>, Error<IOError>]
```

`R` is checked using ordinary type variables and spec constraints. The compiler
does not derive a region from a runtime pathname. Host binding maps the checked
region identity to an explicitly configured directory and permissions. A
well-formed region name, nominal path value or imported spec implementation
does not by itself grant filesystem access. Concrete paths remain operation
payloads and trace data, not new runtime-value type arguments.

Keep three guarantees independent:

| Guarantee | Meaning | Does not imply |
|---|---|---|
| Directory binding | Operations retain the directory object opened at authorization | Frozen contents, a lease, or persistent identity |
| Path confinement | Relative resolution cannot escape the retained root under the backend's documented rules | A sandbox for arbitrary subprocesses, or protection from every hard-link/mount alias |
| Process isolation | An activated OS/runtime backend restricts a child process's actual access | Automatic rollback, application ownership, or remote-side cancellation |

The interpreter and future AIR runtime reuse these mechanisms without sharing
their execution representation. EDK provides source-level helpers; applications
decide workspace provisioning, lease/fencing policy, takeover, recovery,
retention and the association between approvals and conversation turns.

## 2. Identity And Binding

Distinguish three identity domains:

| Identity | Owner and use | Lifetime |
|---|---|---|
| Static region identity | Checked source and its host projection, currently `WorkspaceRegionId`; selects a configured region | Program/package identity, not a directory path |
| Opened directory object | Host retains the actual directory handle; filesystem operations are rooted here | Until the last owner releases the handle |
| Persistent logical workspace identity | Application recovery/provisioning metadata | Defined by the application's persistence and trust model |

A live authorization binding also has an opaque local identity. It denotes a
particular grant-to-opened-root association, not a fourth durable identity.
Cloning that binding preserves authority; independently opening the same path
does not implicitly inherit the old grant.

The implementation shape is:

```rust
#[derive(Clone)]
pub struct WorkspaceRoot {
    binding: Arc<RootBinding>,
}

struct RootBinding {
    directory: cap_std::fs::Dir,
    display_path: PathBuf,
}
```

Fields are private. Reuse the existing `WorkspaceRoot`, `WorkspacePathRef`
(region plus relative path), and bound `WorkspacePath` contracts. The retained
handle is accessible only to host implementation modules. Display paths are
diagnostic locators; neither they nor raw descriptors are language handles or
serialized authority. Local binding equality can use shared binding identity;
do not overload it with filesystem-object equivalence.

The existing `same_file::Handle` comparison must not determine whether an
independently opened root inherits a grant. Its documented platform-dependent
false positives make it unsuitable as a universal authorization proof. Keep
object-comparison functionality separate and internal unless a concrete caller
needs it. Such an API must expose unsupported/indeterminate results, compare
opened objects using a documented platform mechanism, and never silently
substitute path equality. An inode/file ID may be reused; a workspace UUID file
may be copied. Neither alone proves physical continuity across process restarts.

### 2.1 Opening And Registering

```text
trusted host configuration supplies a root locator
  -> open the directory with the selected backend
  -> authorize the actual opened object and required guarantees
  -> publish a region-to-root binding with operation-specific grants
  -> resolve each request through that binding
```

Canonicalization may normalize the initial locator or provide display text; it
is not a security check followed by a later ambient open. If authorization is
relative to an already trusted parent, open relative to that parent. If a
controller approves a candidate root, retain that opened candidate through
approval and publication. A pre-approval path string must not later select a
replacement object.

Move the shared `WorkspaceRegionRegistry` out of `filesystem/local.rs` into
`sandbox/workspace/registry.rs`, retaining its public export. Filesystem and
command clients consume the same immutable registry rather than maintaining
separate binding algorithms. Mutation belongs to configuration construction,
not live request execution.

`insert(region, root)` must validate before mutation and use vacant-entry
insertion. Duplicate registration returns an error and leaves the original
root and grants unchanged. Do not replace an entry and then report an error.
Unknown regions fail with an authority error; there is no current-directory,
project-root or ambient-path fallback. Child-region relationships require
explicit bindings and grants; a source `Within<Parent>` fact is not permission
to invent an OS path mapping.

### 2.2 Rename, Replacement And Reopen

On a platform that permits renaming an opened directory:

```text
/work/project names directory A
WorkspaceRoot opens and binds A
rename /work/project to /work/project.old
create directory B at /work/project
existing binding still operates on A
an explicit reopen of /work/project opens B and needs a new binding
```

Some platforms may instead prevent the rename while a handle is open. Both
behaviors preserve the essential contract: an existing binding must not
silently switch to B. If the old object becomes unusable, return a real error;
do not reopen its remembered path automatically.

Binding does not freeze directory entries. A later lookup can observe changed
contents within the bound root. Once an operation retains a leaf or parent
handle, it operates on that retained object even if its namespace location
changes. This is object-bound access, not a promise that every object stays
under the original pathname forever. Stronger protection against external
writers, directory relocation, mounts or hard links requires an appropriate
deployment/isolation policy, not a path-prefix check.

## 3. Filesystem Operation Contract

The shared path is:

```text
checked Fs action and runtime payload
  -> action admission and operation registration
  -> region registry binding and read/write/delete grant check
  -> normalized relative path
  -> handle-relative resolution and operation
  -> typed result, completion evidence and cleanup report
```

Use the existing `cap_std::fs::Dir` facilities for directory-relative access;
do not hand-roll another resolver or call ambient `std::fs` APIs after
authorization. Reject absolute paths, drive/UNC prefixes and parent traversal
in request paths. Validate symlink resolution at the actual operation boundary;
only a backend that guarantees confinement may follow in-root symlinks.
Unsupported resolution guarantees fail explicitly. Root-directory operations,
where supported by the protocol, must be explicit rather than an empty-path
fallback for malformed input.

`resolve_existing` and `resolve_for_create` return bound path locators, not proof
that a future leaf is unchanged. Implement operations as follows:

| Operation | Required binding and result semantics |
|---|---|
| Read | Open relative to the root, inspect the opened file, and read that handle; reject unsupported special files and enforce operation limits |
| List/stat | Inspect an opened directory/object or a handle-relative entry according to explicit follow/no-follow semantics; do not re-stat an ambient display path |
| Create directories | Create/open components relative to retained parents and retain confinement across each step |
| Delete/rename | Retain parent handle(s), then operate on final entry names; unlinking a symlink must not delete its target |
| Atomic replace | Create a unique temporary file in the bound destination parent, write it, then replace the destination entry relative to that same parent |

Atomic replacement guarantees publication of one directory entry, not a
multi-file transaction, lost-update protection or crash durability. Failure
before publication leaves the old destination intact and cleans the temporary
entry. Failure/cancellation after publication must not report that no write
occurred. Durability requires the backend's documented file/directory flush
sequence; a durability failure after rename reports a committed change with
uncertain durability. Concurrent replaces may be last-writer-wins unless a
separate versioned operation is explicitly provided.

Hard links, mount points and special files need an explicit deployment threat
model. Path confinement alone is not proof that the same data has no aliases
outside the root. Do not advertise cross-mount denial or prevention of all
external modifications unless the selected backend actually enforces it.

## 4. Command And Platform Isolation

`command/local/cwd.rs` binds a child's working directory through the same root
registry and a retained directory handle. On Unix it may use `fchdir` in the
child; it must not mutate the parent process's cwd or reopen a pathname after
checking it. An unsupported handle-bound cwd returns an explicit host error.

This is not process confinement. Program allowlisting, `env_clear`, cwd binding,
process-tree supervision and a configured `Landlock`/`Container`/`WasiPreopen`
label each serve different purposes. None alone proves an arbitrary native
child is restricted to the workspace.

Platform handling must distinguish:

1. Required guarantees from the selected command profile.
2. Backend availability and supported restrictions.
3. Restrictions actually activated for this invocation before untrusted code
   executes.

Only backend-produced activation evidence may satisfy a required guarantee.
`PlatformSandbox::require_hook` must not treat presence in a configuration vector
as proof of enforcement. Reject an unavailable backend or incomplete activation
before executing the command; never retry in an unrestricted mode. A separately
authorized trusted-process mode, if exposed by deployment configuration, must
be explicit and reported as unconfined, not selected on sandbox failure.

Use an explicit child descriptor/handle inheritance allowlist. Root, temporary,
secret and unrelated service handles must not leak across spawn/exec. A
descriptor used to set cwd can be retained for child setup without remaining
inherited by the executed program. WASI preopens are intentional grants and
must be scoped accordingly. Audit both POSIX close-on-exec behavior and Windows
handle inheritance rather than assuming language RAII prevents leakage.

Do not invent one portable `sandboxed: true` flag. Report requested and active
filesystem/network/process guarantees separately, including backend/platform
identity. A missing guarantee maps to a structured host failure using existing
error categories, not successful execution with a warning.

## 5. Lifetime, Recovery And Staging

Reuse [Shared Execution Lifecycle](etas-execution-design.md): an admitted
filesystem or command operation retains its binding until local completion.
Closing an invocation must not invalidate a handle still owned by managed work.
Stopping new work, waiting for existing work, releasing handles and observing
external side effects are distinct events. Cancellation does not revoke a
grant already used by an external process or undo a committed write.

Checkpoint metadata may record a logical workspace reference and diagnostic
binding information. It cannot restore an OS handle or serialize authority.
Resume opens and authorizes a fresh binding using current host configuration;
it must not trust the saved path, region or file ID as sufficient approval.
The application decides whether it accepts that binding as its logical
workspace, whether another worker owns a lease, and whether recovery proceeds.
Etas neither fabricates cross-restart identity nor silently takes over a lease.

`sandbox::snapshot` and `sandbox::diff` describe explicitly staged changes.
An opened directory is not a snapshot. A copied directory is not a consistent
snapshot in the presence of concurrent writers unless a real snapshot or
coordination mechanism supplies that guarantee. Rollback applies only to
changes actually isolated in staging; it cannot undo arbitrary command writes,
network requests or external edits. Publication and conflicts must be reported
truthfully. Multi-file atomic commit and distributed workspace leases are not
implicit features of the local filesystem adapter.

## 6. File Responsibilities And Migration

Target boundaries, reusing existing service modules:

```text
crates/etas_host/src/
  sandbox/
    workspace/
      mod.rs                    # existing workspace public exports
      root.rs                   # retained directory and live binding identity
      path.rs                   # region/path values and relative normalization
      registry.rs               # shared immutable region-to-root bindings
      tests.rs                  # binding, duplicate and reopen contracts
    filesystem.rs               # filesystem policy and handle-relative operations
    filesystem/tests.rs         # real temporary-filesystem race regressions
    command.rs                  # program/profile admission, not process isolation
    platform/
      mod.rs                    # supported backends and narrow public contract
      requirements.rs           # independent required guarantees
      activation.rs             # backend activation and enforced-guarantee evidence
    snapshot.rs                 # explicitly staged change mechanics
    diff.rs                     # audit representation of staged changes
  filesystem/
    protocol.rs                 # std.fs host payloads and typed results
    local.rs                    # request dispatch, admission and managed operation
  command/
    local.rs                    # admitted spawn and backend integration
    local/cwd.rs                # descriptor-bound child cwd
    local/process_tree.rs       # platform process-tree ownership
    local/supervisor.rs         # execution stop, reap and completion reporting
```

Concrete platform backend implementations belong under `sandbox/platform/`
only when implemented and tested. Do not add placeholder backends that report
success. Keep `filesystem.rs` and its tests at their existing boundary; this
change does not require a second filesystem protocol or generic `substrate/`.

Implementation work must preserve existing handle-relative operations and
their race regressions. Replace the path-only root model, OS-object-based grant
equality and destructive duplicate insertion; centralize the registry; extend
platform enforcement/reporting. Remove superseded implementations when moving
responsibilities instead of retaining forwarding dispatch paths. This document
specifies the target, not a claim that all existing adapters satisfy it.

## 7. Acceptance Matrix

Tests use disposable roots and outside sentinels under a test-owned temporary
directory. Use synchronization hooks/barriers for races, hard deadlines and
actual adapters. An unsupported-platform test must assert the structured
rejection, not silently skip the required guarantee or substitute a fake.

| Case | Required observation |
|---|---|
| Root rename and replacement | Old binding continues to address the opened object, or the OS denies rename; it never switches to the replacement |
| Explicit reopen | New binding needs current authorization; neither equal path nor copied UUID inherits the old grant |
| Duplicate region registration | Failure preserves the first root, grants and subsequent operation results |
| Parent/leaf replacement races | Read, stat, list, create, delete and atomic replace use retained objects or reject; no ambient reopen |
| Malicious relative paths | Parent traversal, absolute/drive/UNC paths and escaping symlinks fail without accessing outside sentinels |
| Namespace versus object binding | Tests document permitted in-root replacements and retained-parent behavior without claiming a frozen tree |
| Atomic replace failures | Inject pre-write, pre-rename and post-rename failures; distinguish unchanged destination, committed write and uncertain durability; clean temporary entries |
| Concurrent operations | Independent operations remain correct; concurrent replace follows its documented conflict behavior and cannot escape the root |
| Shared lifetime and cancellation | One cancelled operation does not invalidate another binding owner; pending cleanup remains visible |
| Cwd replacement | Child uses the opened cwd object even if its former path is replaced; parent cwd is unchanged |
| Descriptor inheritance | A real child cannot access an ungranted root/secret/service descriptor; explicit stdin/stdout/WASI grants remain usable |
| Process escape | A sandbox-required child is prevented from accessing test-owned outside paths through absolute paths, traversal or inherited handles; cwd-only mode is not counted as isolation |
| Missing/failed backend | No command executes after activation failure; no success-only hook stub or unrestricted retry |
| Recovery and staging | Reauthorization is required after reopen; cancellation and ordinary snapshots do not claim rollback of externally committed changes |

Run supported guarantee suites on each advertised platform. Deterministic unit
fakes can test policy construction, but they cannot replace real filesystem,
child-process and backend-enforcement evidence. Do not run a potentially
unrestricted malicious process against user files to test failure behavior.

### 7.1 Linux Enforcement Gate

The current required Linux command-isolation contract needs Landlock ABI v5
features. Accept it only on a Linux host/VM/CI runner that probes the actual ABI,
enabled syscalls and all additional mechanisms used by the adapter, including
seccomp where required. Kernel version strings, container images and a configured
backend name are not proof of available or activated enforcement. ABI v5 alone
does not certify every filesystem/network/process guarantee in the matrix.

Maintain a dedicated Linux acceptance job for each advertised architecture,
including x86_64 and arm64. A VM with its own suitable kernel is acceptable;
a container still depends on the host kernel. Portable macOS/unit tests and
unsupported-backend tests remain separate from this enforcement job.

Explicitly run the existing real-adapter suite, including opt-in tests:

```sh
cargo test -p etas_host --test landlock
cargo test -p etas_host --test landlock -- --ignored landlock_
```

The gate verifies the expected test inventory actually executed, rather than
accepting a successful command with zero matching tests. A missing ABI,
unsupported required feature, activation failure, skipped required case or
cleanup timeout fails this gate. Do not convert these outcomes to an ignored
green result. The subprocess-only probe is invoked by its parent test, not as
an unrestricted standalone test.

Required evidence includes authorized read/write/delete success; denied outside
read/write; root rename/replacement and symlink races; network restrictions;
peer-process and inherited-descriptor restrictions; cancellation and bounded
process cleanup; and failed activation/exec without an unrestricted retry.
Use test-owned temporary roots/sentinels and actual isolated child processes.
A deny-all adapter cannot pass by reporting every operation as rejected.

Publish kernel, architecture, probed ABI, requested and activated guarantees,
test names/results and cleanup status with the acceptance artifact. No production
paths or secrets belong in those logs. Runtime adapters continue to fail closed
when a required guarantee is unavailable. An explicitly authorized trusted mode
is a different contract, never a fallback after isolation fails.

Architecture acceptance does not certify the current implementation. Until this
job has real evidence, Linux enforcement remains unaccepted; this does not imply
that unrelated portable storage or lifecycle work cannot proceed.

## 8. Practice References

- [Linux Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html):
  runtime ABI probing and versioned access-right support; enforcement guarantees
  must match the active kernel and the complete adapter isolation contract.
- [Rust cap-std filesystem API](https://docs.rs/cap-std/latest/cap_std/fs/):
  directory-relative authority, reused through the repository's dependency;
  ambient `std::fs` calls are not a substitute for confined resolution.
- [Go os.Root](https://pkg.go.dev/os#Root) and
  [traversal-resistant file APIs](https://go.dev/blog/osroot): opened-root
  operations and race-resistant resolution, with explicit platform and
  mount/special-file limitations rather than universal sandbox claims.
- [Java SecureDirectoryStream](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/file/SecureDirectoryStream.html):
  opened-directory-relative operations where supported. Its availability and
  path/link rules still need checking; it is not a general process sandbox.
- [Java fileKey](https://docs.oracle.com/en/java/javase/25/docs/api/java.base/java/nio/file/attribute/BasicFileAttributes.html#fileKey())
  and [same-file Handle](https://docs.rs/same-file/latest/same_file/struct.Handle.html):
  filesystem identity has availability, reuse and comparison limitations;
  do not promote it to a portable persistent authorization identity.
- [Python descriptor-relative filesystem operations](https://docs.python.org/3/library/os.html#files-and-directories):
  `dir_fd` and link-following controls illustrate the same separation between
  OS mechanisms, platform support and application security policy.
