module std.host.sandbox;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

@std.role("std.host.sandbox.DefaultCommandSandbox")
public let DefaultCommandSandbox: std.host.sandbox.SandboxProfile;

// Compiler-known support type for command sandbox profiles.
public type SandboxProfile;
