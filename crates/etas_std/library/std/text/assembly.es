module std.text;

import std.text.{TextError, char_count, chars, from_char};

public flow concat(parts: Array<string>) -> string {
    var output = "";
    for part in parts limit Iterations(18446744073709551615) {
        output = output + part;
    }
    return output;
}

public flow join(parts: Array<string>, separator: string) -> string {
    var output = "";
    var first = true;
    for part in parts limit Iterations(18446744073709551615) {
        if !first {
            output = output + separator;
        }
        output = output + part;
        first = false;
    }
    return output;
}

public flow repeat(text: string, count: usize) -> string ![Error<TextError>] {
    var output = "";
    var index: usize = 0;
    while index < count limit Iterations(18446744073709551615) {
        output = output + text;
        index = index + 1;
    }
    return output;
}

public flow pad_start(text: string, width: usize, fill: char) -> string {
    let current = char_count(text);
    if current >= width {
        return text;
    }
    var output = "";
    var remaining = width - current;
    while remaining > 0 limit Iterations(18446744073709551615) {
        output = output + from_char(fill);
        remaining = remaining - 1;
    }
    return output + text;
}

public flow pad_end(text: string, width: usize, fill: char) -> string {
    let current = char_count(text);
    if current >= width {
        return text;
    }
    var output = text;
    var remaining = width - current;
    while remaining > 0 limit Iterations(18446744073709551615) {
        output = output + from_char(fill);
        remaining = remaining - 1;
    }
    return output;
}

public flow reverse_chars(text: string) -> string {
    var output = "";
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        output = from_char(scalar) + output;
    }
    return output;
}
