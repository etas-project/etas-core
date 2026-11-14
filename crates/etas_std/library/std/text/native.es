module std.text;

import std.char.is_whitespace;
import std.text.{chars, from_char};

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
