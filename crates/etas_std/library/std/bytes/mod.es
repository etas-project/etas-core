module std.bytes;

// Bytes are stored by the runtime; their storage length is a primitive query.
@std.binding("std.bytes.len")
public flow len(value: bytes) -> usize ![] {}
