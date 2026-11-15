module std.text;

import std.text.{chars, from_char};

public flow ascii_lowercase(text: string) -> string ![] {
    var output = "";
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        let folded = match scalar {
            'A' => 'a',
            'B' => 'b',
            'C' => 'c',
            'D' => 'd',
            'E' => 'e',
            'F' => 'f',
            'G' => 'g',
            'H' => 'h',
            'I' => 'i',
            'J' => 'j',
            'K' => 'k',
            'L' => 'l',
            'M' => 'm',
            'N' => 'n',
            'O' => 'o',
            'P' => 'p',
            'Q' => 'q',
            'R' => 'r',
            'S' => 's',
            'T' => 't',
            'U' => 'u',
            'V' => 'v',
            'W' => 'w',
            'X' => 'x',
            'Y' => 'y',
            'Z' => 'z',
            _ => scalar,
        };
        output = output + from_char(folded);
    }
    return output;
}

public flow ascii_uppercase(text: string) -> string ![] {
    var output = "";
    for scalar in chars(text) limit Iterations(18446744073709551615) {
        let folded = match scalar {
            'a' => 'A',
            'b' => 'B',
            'c' => 'C',
            'd' => 'D',
            'e' => 'E',
            'f' => 'F',
            'g' => 'G',
            'h' => 'H',
            'i' => 'I',
            'j' => 'J',
            'k' => 'K',
            'l' => 'L',
            'm' => 'M',
            'n' => 'N',
            'o' => 'O',
            'p' => 'P',
            'q' => 'Q',
            'r' => 'R',
            's' => 'S',
            't' => 'T',
            'u' => 'U',
            'v' => 'V',
            'w' => 'W',
            'x' => 'X',
            'y' => 'Y',
            'z' => 'Z',
            _ => scalar,
        };
        output = output + from_char(folded);
    }
    return output;
}

public flow eq_ignore_ascii_case(left: string, right: string) -> bool ![] {
    return ascii_lowercase(left) == ascii_lowercase(right);
}
