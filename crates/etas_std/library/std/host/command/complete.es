module std.host.command;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Host command support type.
@std.role("std.host.command.Command")
public type Command;

// An incremental command output or terminal process event.
public enum CommandEvent {
    Exited(i32, bytes, bytes),
    Output(bytes, bytes),
    Stopped(bytes, bytes),
}

// Host command support type.
public type CommandHandle;

// Host command support type.
@std.role("std.host.command.CommandResult")
public type CommandResult = { exit_code: i32, stdout: bytes, stderr: bytes };

// Close a background command's standard input.
@std.binding("std.host.command.close_stdin")
public flow close_stdin(arg0: std.host.command.CommandHandle) -> unit ![] {}

// Construct an opaque command value from a program and argument list.
@std.binding("std.host.command.command")
public flow command(arg0: string, arg1: std.collections.Array<string>) -> std.host.command.Command ![] {}

// Return the stable reconnect identifier for a command handle.
@std.binding("std.host.command.id")
public flow id(arg0: std.host.command.CommandHandle) -> string ![] {}

// Read a bounded incremental output event from a background command.
@std.binding("std.host.command.read")
public flow read(arg0: std.host.command.CommandHandle, arg1: usize) -> std.host.command.CommandEvent ![] {}

// Reconnect to a host-managed background command by stable identifier.
@std.binding("std.host.command.reconnect")
public flow reconnect(arg0: string, arg1: std.host.sandbox.SandboxProfile) -> std.host.command.CommandHandle ![] {}

// Run a command through the checked command host boundary.
@std.binding("std.host.command.run")
public flow run(arg0: std.host.command.Command, arg1: std.host.sandbox.SandboxProfile) -> std.host.command.CommandResult ![] {}

// Start a command as a host-managed background process.
@std.binding("std.host.command.start")
public flow start(arg0: std.host.command.Command, arg1: std.host.sandbox.SandboxProfile) -> std.host.command.CommandHandle ![] {}

// Stop a background command and return its terminal output event.
@std.binding("std.host.command.stop")
public flow stop(arg0: std.host.command.CommandHandle) -> std.host.command.CommandEvent ![] {}

// Return a command value with a region-indexed working directory.
@std.binding("std.host.command.with_cwd")
public flow with_cwd<R ~ std.fs.Region>(arg0: std.host.command.Command, arg1: std.fs.WorkspacePath<R>) -> std.host.command.Command ![] {}

// Return a command value with an explicit environment.
@std.binding("std.host.command.with_env")
public flow with_env(arg0: std.host.command.Command, arg1: std.collections.Map<string, string>) -> std.host.command.Command ![] {}

// Return a command value with bounded stdin bytes.
@std.binding("std.host.command.with_stdin")
public flow with_stdin(arg0: std.host.command.Command, arg1: bytes) -> std.host.command.Command ![] {}

// Write bytes to a background command's standard input.
@std.binding("std.host.command.write")
public flow write(arg0: std.host.command.CommandHandle, arg1: bytes) -> unit ![] {}
