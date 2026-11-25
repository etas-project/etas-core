module std.text;

import std.text.{chars, char_count, from_char, character_array, matches_at, match_position};


public flow splitn(text: string, separator: string, limit: usize) -> Array<string> ![] {
    var output: Array<string> = [];
    if limit == 0 { return output; }
    if limit == 1 { return [text]; }
    let width = char_count(separator);
    var remaining = limit - 1;
    var segment = "";
    if width == 0 {
        output = output.push("");
        remaining = remaining - 1;
        for scalar in chars(text) {
            if remaining > 0 {
                output = output.push(from_char(scalar));
                remaining = remaining - 1;
            } else { segment = segment + from_char(scalar); }
        }
        return output.push(segment);
    }
    let values = character_array(text);
    let expected = character_array(separator);
    let length = char_count(text);
    var index: usize = 0;
    while index < length {
        if remaining > 0 && matches_at(values, expected, index) {
            output = output.push(segment);
            segment = "";
            remaining = remaining - 1;
            index = index + width;
        } else {
            segment = segment + from_char(values.get(index).unwrap());
            index = index + 1;
        }
    }
    return output.push(segment);
}

public flow split(text: string, separator: string) -> Array<string> ![] {
    return splitn(text, separator, char_count(text) + 2);
}

flow partition(text: string, start: usize, width: usize) -> (string, string) ![] {
    var left = "";
    var right = "";
    var index: usize = 0;
    for scalar in chars(text) {
        if index < start { left = left + from_char(scalar); }
        else if index >= start + width { right = right + from_char(scalar); }
        index = index + 1;
    }
    return (left, right);
}

public flow split_once(text: string, separator: string) -> Option<(string, string)> ![] {
    return match match_position(text, separator, false) {
        Some((index, _)) => Some(partition(text, index, char_count(separator))),
        None => None,
    };
}

public flow rsplit_once(text: string, separator: string) -> Option<(string, string)> ![] {
    return match match_position(text, separator, true) {
        Some((index, _)) => Some(partition(text, index, char_count(separator))),
        None => None,
    };
}
