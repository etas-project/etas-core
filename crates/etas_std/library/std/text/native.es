module std.text;

import std.text.{chars, from_char};


public flow char_count(value: string) -> usize ![] {
    var count: usize = 0;
    for scalar in chars(value) {
        count = count + 1;
    }
    return count;
}

public flow is_empty(value: string) -> bool ![] {
    return len(value) == 0;
}

public flow trim_start(value: string) -> string ![] {
    var output = "";
    var started = false;
    for scalar in chars(value) {
        if started || !scalar.is_whitespace() {
            output = output + from_char(scalar);
            started = true;
        }
    }
    return output;
}

public flow trim_end(value: string) -> string ![] {
    var output = "";
    var pending = "";
    for scalar in chars(value) {
        if scalar.is_whitespace() {
            pending = pending + from_char(scalar);
        } else {
            output = output + pending + from_char(scalar);
            pending = "";
        }
    }
    return output;
}

// Whitespace policy and Unicode property lookup both run as Etas code.
public flow trim(value: string) -> string ![] {
    var output = "";
    var pending = "";
    var started = false;
    for scalar in chars(value) {
        if scalar.is_whitespace() {
            if started { pending = pending + from_char(scalar); }
        } else {
            output = output + pending + from_char(scalar);
            pending = "";
            started = true;
        }
    }
    return output;
}
