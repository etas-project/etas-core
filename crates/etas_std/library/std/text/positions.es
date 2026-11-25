module std.text;

import std.core.Index;


public flow len(text: string) -> usize ![] {
    return text.length;
}

public flow is_ascii(text: string) -> bool ![] {
    for byte in text.utf8 {
        if byte >= 128 { return false; }
    }
    return true;
}

public flow char_get<I ~ Index>(text: string, index: I) -> Option<char> ![] {
    let requested = match index::Index.to_usize() {
        Some(value) => value,
        None => { return None; },
    };
    var current: usize = 0;
    for scalar in chars(text) {
        if current == requested { return Some(scalar); }
        current = current + 1;
    }
    return None;
}

public flow is_char_boundary<I ~ Index>(text: string, index: I) -> bool ![] {
    let byte = match index::Index.to_usize() {
        Some(value) => value,
        None => { return false; },
    };
    if byte == text.length { return true; }
    return match text.utf8.get(byte) {
        Some(value) => value < 128 || value >= 192,
        None => false,
    };
}
