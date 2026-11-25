module std.text;

import std.text.{chars, char_count, from_char};


// Scalar order identifies the four UTF-8 encoding widths.
// U+0080, U+0800 and U+10000 begin the longer forms.
flow scalar_width(scalar: char) -> usize ![] {
    if scalar < '' { return 1; }
    if scalar < 'ࠀ' { return 2; }
    if scalar < '𐀀' { return 3; }
    return 4;
}

flow character_array(text: string) -> Array<char> ![] {
    var output: Array<char> = [];
    for scalar in chars(text) {
        output = output.push(scalar);
    }
    return output;
}

flow matches_at(text: Array<char>, pattern: Array<char>, start: usize) -> bool ![] {
    var index = start;
    for expected in pattern {
        match text.get(index) {
            Some(actual) => { if actual != expected { return false; } },
            None => { return false; },
        };
        index = index + 1;
    }
    return true;
}

flow match_position(text: string, pattern: string, last: bool) -> Option<(usize, usize)> ![] {
    let values = character_array(text);
    let expected = character_array(pattern);
    let length = char_count(text);
    var index: usize = 0;
    var byte: usize = 0;
    var found: Option<(usize, usize)> = None;
    while index <= length {
        if matches_at(values, expected, index) {
            if !last { return Some((index, byte)); }
            found = Some((index, byte));
        }
        if index == length { return found; }
        byte = byte + scalar_width(values.get(index).unwrap());
        index = index + 1;
    }
    return found;
}

public flow find(text: string, pattern: string) -> Option<usize> ![] {
    return match match_position(text, pattern, false) {
        Some((_, byte)) => Some(byte), None => None,
    };
}

public flow rfind(text: string, pattern: string) -> Option<usize> ![] {
    return match match_position(text, pattern, true) {
        Some((_, byte)) => Some(byte), None => None,
    };
}

public flow count(text: string, pattern: string) -> usize ![] {
    let pattern_length = char_count(pattern);
    if pattern_length == 0 { return char_count(text) + 1; }
    let values = character_array(text);
    let expected = character_array(pattern);
    let length = char_count(text);
    var index: usize = 0;
    var total: usize = 0;
    while index < length {
        if matches_at(values, expected, index) {
            total = total + 1;
            index = index + pattern_length;
        } else { index = index + 1; }
    }
    return total;
}

public flow starts_with(text: string, prefix: string) -> bool ![] {
    return matches_at(character_array(text), character_array(prefix), 0);
}

public flow ends_with(text: string, suffix: string) -> bool ![] {
    let length = char_count(text);
    let suffix_length = char_count(suffix);
    if suffix_length > length { return false; }
    return matches_at(character_array(text), character_array(suffix), length - suffix_length);
}

public flow contains(text: string, pattern: string) -> bool ![] {
    let values = character_array(text);
    let expected = character_array(pattern);
    let length = char_count(text);
    var start: usize = 0;
    while start <= length {
        if matches_at(values, expected, start) { return true; }
        start = start + 1;
    }
    return false;
}

public flow strip_prefix(text: string, prefix: string) -> Option<string> ![] {
    if !starts_with(text, prefix) { return None; }
    var remaining = char_count(prefix);
    var output = "";
    for scalar in chars(text) {
        if remaining > 0 {
            remaining = remaining - 1;
        } else {
            output = output + from_char(scalar);
        }
    }
    return Some(output);
}

public flow strip_suffix(text: string, suffix: string) -> Option<string> ![] {
    if !ends_with(text, suffix) { return None; }
    var remaining = char_count(text) - char_count(suffix);
    var output = "";
    for scalar in chars(text) {
        if remaining == 0 { return Some(output); }
        output = output + from_char(scalar);
        remaining = remaining - 1;
    }
    return Some(output);
}
