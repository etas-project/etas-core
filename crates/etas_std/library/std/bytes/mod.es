module std.bytes;

public flow len(value: bytes) -> usize ![] {
    return value.length;
}
