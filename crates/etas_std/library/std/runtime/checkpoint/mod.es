module std.runtime.checkpoint;

@std.binding("std.runtime.checkpoint")
public flow checkpoint<T>(label: T) -> unit ![] {}
