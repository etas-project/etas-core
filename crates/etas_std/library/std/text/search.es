module std.text;

import std.collections.{array_get, array_push};
import std.text.{chars, char_count, from_char};

flow character_array(text: string) -> Array<char> ![] {
    var output: Array<char> = [];
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        output = array_push(output, scalar);
    }
    return output;
}

flow matches_at(text: Array<char>, pattern: Array<char>, start: usize) -> bool ![] {
    var index = start;
    for expected in pattern limit Iterations(18446744073709551615) {
        match array_get(text, index) {
            Some(actual) => { if actual != expected { return false; } },
            None => { return false; },
        };
        index = index + 1;
    }
    return true;
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
    while start <= length limit Iterations(18446744073709551615) {
        if matches_at(values, expected, start) { return true; }
        start = start + 1;
    }
    return false;
}

public flow strip_prefix(text: string, prefix: string) -> Option<string> ![] {
    if !starts_with(text, prefix) { return None; }
    var remaining = char_count(prefix);
    var output = "";
    for scalar in chars(text) limit Iterations(18446744073709551615) {
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
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        if remaining == 0 { return Some(output); }
        output = output + from_char(scalar);
        remaining = remaining - 1;
    }
    return Some(output);
}
