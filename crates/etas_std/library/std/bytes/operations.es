module std.bytes;

public flow empty() -> bytes ![] { return bytes { data= [] }; }
public flow from_array(values: Array<u8>) -> bytes ![] { return bytes { data= values }; }
public flow to_array(value: bytes) -> Array<u8> ![] {
    var output: Array<u8> = [];
    for byte in value { output = output.push(byte); }
    return output;
}
public flow is_empty(value: bytes) -> bool ![] { return value.length == 0; }
public flow get<I ~ Index>(value: bytes, index: I) -> Option<u8> ![] {
    return match value[index]? { Ok(byte) => Some(byte), Err(_) => None };
}
public flow at<I ~ Index>(value: bytes, index: I) -> u8 ![Error<std.runtime.error.IndexError>] { return value[index]; }
public flow slice<I ~ Index>(value: bytes, range: Range<I>) -> bytes ![Error<std.runtime.error.IndexError>] {
    var start = match range.start.to_usize() { Some(offset) => offset, None => usize.maximum() };
    var end = match range.end.to_usize() { Some(offset) => offset, None => usize.maximum() };
    if !range.include_start {
        if start >= value.length {
            perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= start, length= value.length });
        }
        start = start + 1;
    }
    if range.include_end {
        if end >= value.length {
            perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= end, length= value.length });
        }
        end = end + 1;
    }
    if start > end || end > value.length {
        perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= if start > end { start } else { end }, length= value.length });
    }
    return value[start, end);
}
public flow iter(value: bytes) -> Iter<u8> ![] { return value.iter(); }
public flow concat(parts: Array<bytes>) -> bytes ![] {
    var output = BytesBuilder.new();
    for part in parts { output.append(part); }
    return output.build();
}
public flow join(parts: Array<bytes>, separator: bytes) -> bytes ![] {
    var output = BytesBuilder.new();
    var first = true;
    for part in parts {
        if !first { output.append(separator); }
        output.append(part); first = false;
    }
    return output.build();
}
public flow starts_with(value: bytes, prefix: bytes) -> bool ![] {
    if prefix.length > value.length { return false; }
    var offset: usize = 0;
    for byte in prefix {
        if value.get(offset).unwrap() != byte { return false; }
        offset = offset + 1;
    }
    return true;
}
public flow ends_with(value: bytes, suffix: bytes) -> bool ![] {
    if suffix.length > value.length { return false; }
    var offset = value.length - suffix.length;
    for byte in suffix {
        if value.get(offset).unwrap() != byte { return false; }
        offset = offset + 1;
    }
    return true;
}

type BytePattern = { private pattern: bytes, private fallback: Array<usize> };
impl BytePattern {
    flow new(pattern: bytes) -> BytePattern ![] {
        var fallback = Array.repeat(pattern.length, 0);
        var border: usize = 0;
        var offset: usize = 1;
        while offset < pattern.length {
            let byte = pattern.get(offset).unwrap();
            while border > 0 && byte != pattern.get(border).unwrap() {
                border = fallback.get(border - 1).unwrap();
            }
            if byte == pattern.get(border).unwrap() { border = border + 1; }
            fallback = fallback.set(offset, border)?.unwrap();
            offset = offset + 1;
        }
        return BytePattern { pattern= pattern, fallback= fallback };
    }
    flow search(self, value: bytes, last: bool) -> Option<usize> ![] {
        var matched: usize = 0;
        var offset: usize = 0;
        var found: Option<usize> = None;
        for byte in value {
            while matched > 0 && byte != self.pattern.get(matched).unwrap() {
                matched = self.fallback.get(matched - 1).unwrap();
            }
            if byte == self.pattern.get(matched).unwrap() { matched = matched + 1; }
            offset = offset + 1;
            if matched == self.pattern.length {
                let start = offset - matched;
                if !last { return Some(start); }
                found = Some(start);
                matched = self.fallback.get(matched - 1).unwrap();
            }
        }
        return found;
    }
}
public flow find(value: bytes, pattern: bytes) -> Option<usize> ![] {
    if pattern.length == 0 { return Some(0); }
    return BytePattern.new(pattern).search(value, false);
}
public flow rfind(value: bytes, pattern: bytes) -> Option<usize> ![] {
    if pattern.length == 0 { return Some(value.length); }
    return BytePattern.new(pattern).search(value, true);
}
public flow contains(value: bytes, pattern: bytes) -> bool ![] {
    if pattern.length == 0 { return true; }
    return BytePattern.new(pattern).search(value, false).is_some();
}

impl bytes {
    public flow at<I ~ Index>(self, index: I) -> u8 ![Error<std.runtime.error.IndexError>] {
        return self[index];
    }
}
