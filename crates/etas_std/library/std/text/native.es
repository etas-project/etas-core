module std.text;

// The implementation is the toolchain's UTF-8 text primitive. The public
// signature and effect contract are maintained here and checked against its ABI.
@std.binding("std.text.trim")
public flow trim(value: string) -> string ![] {}
