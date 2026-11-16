module std.stream;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Bounded stream operation configuration.
public type ByteLimit = { bytes: i32 };

// Standard byte-stream marker spec.
@std.role("std.stream.ByteStream")
public spec ByteStream;

// Typed stream operation failure.
public enum StreamError {
    Cancelled,
    Closed,
    Host(string),
    Interrupted,
    LimitExceeded,
    TimedOut,
}

// A byte chunk or ordinary end of stream.
public enum StreamRead {
    Data(bytes),
    Eof,
}

// Bounded stream operation configuration.
public type Timeout = { ms: i32 };

// Close a host-mediated stream.
@std.binding("std.stream.close")
public flow close<S ~ std.stream.ByteStream>(arg0: S) -> unit ![Error<std.stream.StreamError>] {}

// Flush a host-mediated stream.
@std.binding("std.stream.flush")
public flow flush<S ~ std.stream.ByteStream>(arg0: S) -> unit ![Error<std.stream.StreamError>] {}

// Read at most the requested byte count from a host-mediated stream.
@std.binding("std.stream.read")
public flow read<S ~ std.stream.ByteStream>(arg0: S, arg1: usize, arg2: std.option.Option<std.stream.Timeout>) -> std.stream.StreamRead ![Error<std.stream.StreamError>] {}

// Read from a host-mediated stream until EOF or the requested byte limit.
@std.binding("std.stream.read_until_limit")
public flow read_until_limit<S ~ std.stream.ByteStream>(arg0: S, arg1: std.stream.ByteLimit, arg2: std.option.Option<std.stream.Timeout>) -> bytes ![Error<std.stream.StreamError>] {}

// Write all bytes to a host-mediated stream.
@std.binding("std.stream.write_all")
public flow write_all<S ~ std.stream.ByteStream>(arg0: S, arg1: bytes) -> unit ![Error<std.stream.StreamError>] {}
