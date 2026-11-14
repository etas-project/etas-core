module std.char;

// Unicode property tables are provided by the host character primitive.
@std.binding("std.char.is_whitespace")
public flow is_whitespace(value: char) -> bool ![] {}
