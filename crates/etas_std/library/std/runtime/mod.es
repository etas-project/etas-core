module std.runtime;

@std.binding("std.runtime.checkpoint")
public flow checkpoint<T>(label: T) -> unit ![] {}
