module std.host.sandbox;

public let DefaultCommandSandbox: SandboxProfile = SandboxProfile { name= "default" };

// A profile names a host-configured policy; it does not carry the permission
// to widen that policy. The host checks the selected profile for each operation.
public type SandboxProfile = { name: string };
