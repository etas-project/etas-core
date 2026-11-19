module std.runtime.checkpoint;

@std.role("std.runtime.checkpoint")
public flow checkpoint<T>(label: T) -> unit ![];
