module std.text;

import std.char.is_whitespace;
import std.text.{chars, from_char};

public flow char_count(value: string) -> usize ![] {
    var count: usize = 0;
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        count = count + 1;
    }
    return count;
}

public flow is_empty(value: string) -> bool ![] {
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        return false;
    }
    return true;
}

public flow trim_start(value: string) -> string ![] {
    var output = "";
    var started = false;
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        if started || !is_whitespace(scalar) {
            output = output + from_char(scalar);
            started = true;
        }
    }
    return output;
}

public flow trim_end(value: string) -> string ![] {
    var output = "";
    var pending = "";
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        if is_whitespace(scalar) {
            pending = pending + from_char(scalar);
        } else {
            output = output + pending + from_char(scalar);
            pending = "";
        }
    }
    return output;
}

// Keep text policy in Etas. Only the Unicode scalar property is primitive.
public flow trim(value: string) -> string ![] {
    var output = "";
    var pending = "";
    var started = false;
    for scalar in chars(value) limit Iterations(18446744073709551615) {
        if is_whitespace(scalar) {
            if started { pending = pending + from_char(scalar); }
        } else {
            output = output + pending + from_char(scalar);
            pending = "";
            started = true;
        }
    }
    return output;
}
