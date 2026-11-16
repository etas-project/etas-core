module std.text;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Return a checked UTF-8 byte interval.
@std.binding("std.text.byte_slice")
public flow byte_slice<I ~ std.core.Index>(arg0: string, arg1: std.collections.Range<I>) -> string ![Error<std.text.TextError>] {}

// Lazily traverse byte positions and Unicode scalars.
@std.binding("std.text.char_indices")
public flow char_indices(arg0: string) -> std.collections.Iter<(usize, char)> ![] {}

// Return a checked Unicode scalar interval.
@std.binding("std.text.char_slice")
public flow char_slice<I ~ std.core.Index>(arg0: string, arg1: std.collections.Range<I>) -> string ![Error<std.text.TextError>] {}

// Lazily traverse non-overlapping exact matches.
@std.binding("std.text.find_all")
public flow find_all(arg0: string, arg1: string) -> std.collections.Iter<(usize, usize)> ![] {}

// Find from a checked UTF-8 byte boundary.
@std.binding("std.text.find_from")
public flow find_from<I ~ std.core.Index>(arg0: string, arg1: string, arg2: I) -> std.option.Option<usize> ![Error<std.text.TextError>] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_f32")
public flow from_f32(arg0: f32) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_f64")
public flow from_f64(arg0: f64) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_i128")
public flow from_i128(arg0: i128) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_i16")
public flow from_i16(arg0: i16) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_i32")
public flow from_i32(arg0: i32) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_i64")
public flow from_i64(arg0: i64) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_i8")
public flow from_i8(arg0: i8) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_isize")
public flow from_isize(arg0: isize) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_u128")
public flow from_u128(arg0: u128) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_u16")
public flow from_u16(arg0: u16) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_u32")
public flow from_u32(arg0: u32) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_u64")
public flow from_u64(arg0: u64) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_u8")
public flow from_u8(arg0: u8) -> string ![] {}

// Render a numeric value without narrowing conversion.
@std.binding("std.text.from_usize")
public flow from_usize(arg0: usize) -> string ![] {}

// Return an optional UTF-8 byte interval.
@std.binding("std.text.get_byte_slice")
public flow get_byte_slice<I ~ std.core.Index>(arg0: string, arg1: std.collections.Range<I>) -> std.option.Option<string> ![] {}

// Return an optional Unicode scalar interval.
@std.binding("std.text.get_char_slice")
public flow get_char_slice<I ~ std.core.Index>(arg0: string, arg1: std.collections.Range<I>) -> std.option.Option<string> ![] {}

// Lazily traverse LF or CRLF-delimited lines.
@std.binding("std.text.lines")
public flow lines(arg0: string) -> std.collections.Iter<string> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_f32")
public flow parse_f32(arg0: string) -> std.result.Result<f32, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_f64")
public flow parse_f64(arg0: string) -> std.result.Result<f64, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_i128")
public flow parse_i128(arg0: string) -> std.result.Result<i128, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_i16")
public flow parse_i16(arg0: string) -> std.result.Result<i16, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_i32")
public flow parse_i32(arg0: string) -> std.result.Result<i32, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_i64")
public flow parse_i64(arg0: string) -> std.result.Result<i64, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_i8")
public flow parse_i8(arg0: string) -> std.result.Result<i8, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_isize")
public flow parse_isize(arg0: string) -> std.result.Result<isize, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_u128")
public flow parse_u128(arg0: string) -> std.result.Result<u128, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_u16")
public flow parse_u16(arg0: string) -> std.result.Result<u16, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_u32")
public flow parse_u32(arg0: string) -> std.result.Result<u32, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_u64")
public flow parse_u64(arg0: string) -> std.result.Result<u64, std.text.TextError> ![] {}

// Parse a complete locale-independent numeric string.
@std.binding("std.text.parse_u8")
public flow parse_u8(arg0: string) -> std.result.Result<u8, std.text.TextError> ![] {}

// Replace a checked UTF-8 byte interval.
@std.binding("std.text.replace_range")
public flow replace_range<I ~ std.core.Index>(arg0: string, arg1: std.collections.Range<I>, arg2: string) -> string ![Error<std.text.TextError>] {}

// Lazily split exact separators.
@std.binding("std.text.split_iter")
public flow split_iter(arg0: string, arg1: string) -> std.collections.Iter<string> ![] {}

// Lazily split Unicode White_Space runs.
@std.binding("std.text.split_whitespace")
public flow split_whitespace(arg0: string) -> std.collections.Iter<string> ![] {}
