module std.text;

// Scalar traversal and storage conversion are native primitives. Public names
// and signatures belong to source, including these former bootstrap-only APIs.
@std.binding("std.text.chars")
public flow chars(value: string) -> Iter<char> ![] {}

@std.binding("std.text.from_char")
public flow from_char(value: char) -> string ![] {}

@std.binding("std.text.parse_usize")
public flow parse_usize(value: string) -> Result<usize, TextError> ![] {}
