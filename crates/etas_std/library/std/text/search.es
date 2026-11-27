module std.text;

flow scalar_width(scalar: char) -> usize ![] {
    if scalar.bits < 128 { return 1; }
    if scalar.bits < 2048 { return 2; }
    if scalar.bits < 65536 { return 3; }
    return 4;
}

public flow find(text: string, pattern: string) -> Option<usize> ![] {
    if pattern.length == 0 { return Some(0); }
    return match BytePattern.new(pattern).find(text.utf8, 0) {
        Some((start, _)) => Some(start), None => None,
    };
}
public flow rfind(text: string, pattern: string) -> Option<usize> ![] {
    if pattern.length == 0 { return Some(text.length); }
    return match BytePattern.new(pattern).last(text.utf8) {
        Some((start, _)) => Some(start), None => None,
    };
}
public flow count(text: string, pattern: string) -> usize ![] {
    if pattern.length == 0 { return char_count(text) + 1; }
    let matcher = BytePattern.new(pattern);
    let input = text.utf8;
    var start: usize = 0;
    var total: usize = 0;
    while true {
        match matcher.find(input, start) {
            Some((_, end)) => { total = total + 1; start = end; },
            None => { return total; },
        };
    }
    return total;
}
public flow starts_with(text: string, prefix: string) -> bool ![] {
    if prefix.length > text.length { return false; }
    if prefix.length == 0 { return true; }
    return std.bytes.starts_with(text.utf8, prefix.utf8);
}
public flow ends_with(text: string, suffix: string) -> bool ![] {
    if suffix.length > text.length { return false; }
    if suffix.length == 0 { return true; }
    return std.bytes.ends_with(text.utf8, suffix.utf8);
}
public flow contains(text: string, pattern: string) -> bool ![] {
    if pattern.length == 0 { return true; }
    if pattern.length > text.length { return false; }
    return BytePattern.new(pattern).find(text.utf8, 0).is_some();
}
public flow strip_prefix(text: string, prefix: string) -> Option<string> ![] {
    if prefix.length == 0 { return Some(text); }
    if prefix.length > text.length { return None; }
    let input = text.utf8;
    if !std.bytes.starts_with(input, prefix.utf8) { return None; }
    return Some(TextSpan { input= input, start= prefix.length, end= input.length }.materialize());
}
public flow strip_suffix(text: string, suffix: string) -> Option<string> ![] {
    if suffix.length == 0 { return Some(text); }
    if suffix.length > text.length { return None; }
    let input = text.utf8;
    if !std.bytes.ends_with(input, suffix.utf8) { return None; }
    return Some(TextSpan { input= input, start= 0, end= input.length - suffix.length }.materialize());
}
