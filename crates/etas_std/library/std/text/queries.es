module std.text;

public flow len(value: string) -> usize ![] {
    return std.collections.len(value);
}

public flow contains(value: string, pattern: string) -> bool ![] {
    return find(value, pattern) != None;
}
