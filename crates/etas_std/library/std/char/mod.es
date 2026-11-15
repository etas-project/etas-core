module std.char;

// Unicode property tables are provided by the host character primitive.
@std.binding("std.char.is_whitespace")
public flow is_whitespace(value: char) -> bool ![] {}

@std.binding("std.char.is_alphabetic")
public flow is_alphabetic(value: char) -> bool ![] {}

@std.binding("std.char.is_alphanumeric")
public flow is_alphanumeric(value: char) -> bool ![] {}

@std.binding("std.char.to_u32")
public flow to_u32(value: char) -> u32 ![] {}

@std.binding("std.char.from_u32")
public flow from_u32(value: u32) -> Option<char> ![] {}

public flow is_ascii_digit(value: char) -> bool ![] {
    let scalar = to_u32(value);
    return scalar >= 48 && scalar <= 57;
}

public flow is_control(value: char) -> bool ![] {
    let scalar = to_u32(value);
    return scalar < 32 || (scalar >= 127 && scalar <= 159);
}
