module std.effects;

import std.fs.{Region, WorkspacePath, FsStat};

// Rust binds execution tags and host capabilities; source owns declarations.

@std.role("std.effects.Agentic")
public effect Agentic {
    action infer<C: Agent, O>(arg0: std.agent.prompt.Prompt, arg1: std.agent.schema.Schema<O>) -> O;
}

@std.role("std.effects.Network")
public effect Network;

@std.role("std.effects.FileIO")
public effect FileIO;

@std.role("std.effects.Console")
public effect Console extends FileIO {
    action stdin_read_line() -> string;
    action stdin_read_all() -> string;
    action stdout_write(text: string) -> unit;
    action stderr_write(text: string) -> unit;
}

@std.role("std.effects.Command")
public effect Command {
    action read<Capability0: std.host.command.CommandHandle>(arg0: std.host.command.CommandHandle) -> std.host.command.CommandEvent;
    action reconnect<Capability0: std.host.sandbox.SandboxProfile>(arg0: string) -> std.host.command.CommandHandle;
    action run<Capability0: std.host.sandbox.SandboxProfile>(arg0: std.host.command.Command) -> std.host.command.CommandResult;
    action start<Capability0: std.host.sandbox.SandboxProfile>(arg0: std.host.command.Command) -> std.host.command.CommandHandle;
    action stop<Capability0: std.host.command.CommandHandle>(arg0: std.host.command.CommandHandle) -> std.host.command.CommandEvent;
    action write<Capability0: std.host.command.CommandHandle>(arg0: std.host.command.CommandHandle) -> unit;
}

@std.role("std.effects.Memory")
public effect Memory {
    action<R> read<Place: MemoryPlace>(arg0: std.memory.MemoryRegion<R>) -> unit;
    action<R> write<Place: MemoryPlace>(arg0: std.memory.MemoryRegion<R>) -> unit;
}

@std.role("std.effects.Secret")
public effect Secret {
    action read<K>(arg0: std.secret.SecretKey<K>) -> std.secret.SecretValue<K>;
    action use<K>(arg0: std.secret.SecretValue<K>) -> unit;
}

@std.role("std.effects.Time")
public effect Time;

@std.role("std.effects.Human")
public effect Human;

@std.role("std.effects.Error")
public effect Error<Failure> {
    action raise<E>(error: E) -> never;
}

@std.role("std.effects.Fs")
public effect Fs extends FileIO {
    action read<R ~ Region>(path: WorkspacePath<R>) -> bytes;
    action write<R ~ Region>(path: WorkspacePath<R>) -> unit;
    action list<R ~ Region>(path: WorkspacePath<R>) -> List<WorkspacePath<R>>;
    action stat<R ~ Region>(path: WorkspacePath<R>) -> FsStat;
    action atomic_replace<R ~ Region>(path: WorkspacePath<R>) -> unit;
}
