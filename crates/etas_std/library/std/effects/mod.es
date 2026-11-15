module std.effects;

// Rust binds execution tags and host capabilities; source owns declarations.

@std.role("std.effects.Agentic")
public effect Agentic;

@std.role("std.effects.Network")
public effect Network;

@std.role("std.effects.FileIO")
public effect FileIO;

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
public effect Error<Failure>;

@std.role("std.effects.Fs")
public effect Fs extends FileIO;
