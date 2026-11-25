module std.text;

import std.text.{TextError, char_count, chars};

public flow concat(parts: Array<string>) -> string ![] {
    var output = TextBuilder.new();
    for part in parts {
        output.append(part);
    }
    return output.build();
}

public flow join(parts: Array<string>, separator: string) -> string ![] {
    var output = TextBuilder.new();
    var first = true;
    for part in parts {
        if !first { output.append(separator); }
        output.append(part);
        first = false;
    }
    return output.build();
}

public flow repeat(text: string, count: usize) -> string ![Error<TextError>] {
    if text.length == 0 || count == 0 { return ""; }
    if count > usize.maximum() / text.length {
        perform Error.raise<TextError>(TextError.NumericOverflow);
    }
    var output = TextBuilder.new();
    var index: usize = 0;
    while index < count {
        output.append(text);
        index = index + 1;
    }
    return output.build();
}

public flow pad_start(text: string, width: usize, fill: char) -> string ![] {
    let current = char_count(text);
    if current >= width { return text; }
    var output = TextBuilder.new();
    var remaining = width - current;
    while remaining > 0 {
        output.append_char(fill);
        remaining = remaining - 1;
    }
    output.append(text);
    return output.build();
}

public flow pad_end(text: string, width: usize, fill: char) -> string ![] {
    let current = char_count(text);
    if current >= width { return text; }
    var output = TextBuilder.new();
    output.append(text);
    var remaining = width - current;
    while remaining > 0 {
        output.append_char(fill);
        remaining = remaining - 1;
    }
    return output.build();
}

public flow reverse_chars(text: string) -> string ![] {
    var reversed: List<char> = [];
    for scalar in chars(text) {
        reversed = scalar :: reversed;
    }
    var output = TextBuilder.new();
    for scalar in reversed {
        output.append_char(scalar);
    }
    return output.build();
}
