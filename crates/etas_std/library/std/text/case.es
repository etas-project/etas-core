module std.text;

// Unicode case mappings use the runtime's versioned Unicode data.
@std.binding("std.text.lowercase")
public flow lowercase(value: string) -> string ![] {}

@std.binding("std.text.uppercase")
public flow uppercase(value: string) -> string ![] {}
