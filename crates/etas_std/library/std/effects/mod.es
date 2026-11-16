module std.effects;

import std.fs.{Region, WorkspacePath, FsStat};

// Rust binds execution tags and host capabilities; source owns declarations.

@std.role("std.effects.Agentic")
public effect Agentic;

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
public effect Command;

@std.role("std.effects.Memory")
public effect Memory;

@std.role("std.effects.Secret")
public effect Secret;

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
