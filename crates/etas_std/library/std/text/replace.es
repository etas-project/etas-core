module std.text;

import std.text.{chars, char_count, from_char, character_array, matches_at};


public flow replace_n(text: string, pattern: string, replacement: string, limit: usize) -> string ![] {
    if limit == 0 { return text; }
    let width = char_count(pattern);
    var remaining = limit;
    var output = "";
    if width == 0 {
        output = replacement;
        remaining = remaining - 1;
        for scalar in chars(text) limit Iterations(18446744073709551615) {
            output = output + from_char(scalar);
            if remaining > 0 {
                output = output + replacement;
                remaining = remaining - 1;
            }
        }
        return output;
    }
    let values = character_array(text);
    let expected = character_array(pattern);
    let length = char_count(text);
    var index: usize = 0;
    while index < length limit Iterations(18446744073709551615) {
        if remaining > 0 && matches_at(values, expected, index) {
            output = output + replacement;
            remaining = remaining - 1;
            index = index + width;
        } else {
            output = output + from_char(values.get(index).unwrap());
            index = index + 1;
        }
    }
    return output;
}

public flow replace(text: string, pattern: string, replacement: string) -> string ![] {
    return replace_n(text, pattern, replacement, char_count(text) + 1);
}
