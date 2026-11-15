module std.text;

import std.core.Index;
import std.collections.{array_get, array_push};
import std.text.{chars, scalar_width, character_array};

public flow len(text: string) -> usize ![] {
    var length: usize = 0;
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        length = length + scalar_width(scalar);
    }
    return length;
}

public flow is_ascii(text: string) -> bool ![] {
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        if scalar >= '' { return false; }
    }
    return true;
}

public flow char_get<I ~ Index>(text: string, index: I) -> Option<char> ![] {
    return array_get(character_array(text), index);
}

public flow is_char_boundary<I ~ Index>(text: string, index: I) -> bool ![] {
    var boundaries: Array<bool> = [true];
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        var remaining = scalar_width(scalar);
        while remaining > 1 limit Iterations(18446744073709551615) {
            boundaries = array_push(boundaries, false);
            remaining = remaining - 1;
        }
        boundaries = array_push(boundaries, true);
    }
    return match array_get(boundaries, index) { Some(boundary) => boundary, None => false };
}
