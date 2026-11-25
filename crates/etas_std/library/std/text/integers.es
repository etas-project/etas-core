module std.text;

import std.text.TextError.{NumericOverflow, ParseFailure};

let DECIMAL_DIGITS: Array<string> = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

public flow from_i8(value: i8) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        var digit = remaining % 10;
        if digit < 0 {
            digit = 0 - digit;
        }
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    if value < 0 {
        output = "-" + output;
    }
    return output;
}

public flow parse_i8(text: string) -> Result<i8, TextError> ![] {
    let maximum: i8 = 127;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var negative = false;
    var minimum = 0 - maximum;
    var value: i8 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' {
                negative = true;
                minimum = 0 - maximum - 1;
                continue;
            }
        }
        let digit: i8 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value < minimum / 10 || (value == minimum / 10 && digit > 0 - (minimum % 10)) {
                overflow = true;
            } else {
                value = value * 10 - digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(if negative { value } else { 0 - value });
}

public flow from_i16(value: i16) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        var digit = remaining % 10;
        if digit < 0 {
            digit = 0 - digit;
        }
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    if value < 0 {
        output = "-" + output;
    }
    return output;
}

public flow parse_i16(text: string) -> Result<i16, TextError> ![] {
    let maximum: i16 = 32767;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var negative = false;
    var minimum = 0 - maximum;
    var value: i16 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' {
                negative = true;
                minimum = 0 - maximum - 1;
                continue;
            }
        }
        let digit: i16 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value < minimum / 10 || (value == minimum / 10 && digit > 0 - (minimum % 10)) {
                overflow = true;
            } else {
                value = value * 10 - digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(if negative { value } else { 0 - value });
}

public flow from_i32(value: i32) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        var digit = remaining % 10;
        if digit < 0 {
            digit = 0 - digit;
        }
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    if value < 0 {
        output = "-" + output;
    }
    return output;
}

public flow parse_i32(text: string) -> Result<i32, TextError> ![] {
    let maximum: i32 = 2147483647;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var negative = false;
    var minimum = 0 - maximum;
    var value: i32 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' {
                negative = true;
                minimum = 0 - maximum - 1;
                continue;
            }
        }
        let digit: i32 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value < minimum / 10 || (value == minimum / 10 && digit > 0 - (minimum % 10)) {
                overflow = true;
            } else {
                value = value * 10 - digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(if negative { value } else { 0 - value });
}

public flow from_i64(value: i64) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        var digit = remaining % 10;
        if digit < 0 {
            digit = 0 - digit;
        }
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    if value < 0 {
        output = "-" + output;
    }
    return output;
}

public flow parse_i64(text: string) -> Result<i64, TextError> ![] {
    let maximum: i64 = 9223372036854775807;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var negative = false;
    var minimum = 0 - maximum;
    var value: i64 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' {
                negative = true;
                minimum = 0 - maximum - 1;
                continue;
            }
        }
        let digit: i64 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value < minimum / 10 || (value == minimum / 10 && digit > 0 - (minimum % 10)) {
                overflow = true;
            } else {
                value = value * 10 - digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(if negative { value } else { 0 - value });
}

public flow from_i128(value: i128) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        var digit = remaining % 10;
        if digit < 0 {
            digit = 0 - digit;
        }
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    if value < 0 {
        output = "-" + output;
    }
    return output;
}

public flow parse_i128(text: string) -> Result<i128, TextError> ![] {
    let maximum: i128 = 170141183460469231731687303715884105727;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var negative = false;
    var minimum = 0 - maximum;
    var value: i128 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' {
                negative = true;
                minimum = 0 - maximum - 1;
                continue;
            }
        }
        let digit: i128 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value < minimum / 10 || (value == minimum / 10 && digit > 0 - (minimum % 10)) {
                overflow = true;
            } else {
                value = value * 10 - digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(if negative { value } else { 0 - value });
}

public flow from_isize(value: isize) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        var digit = remaining % 10;
        if digit < 0 {
            digit = 0 - digit;
        }
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    if value < 0 {
        output = "-" + output;
    }
    return output;
}

public flow parse_isize(text: string) -> Result<isize, TextError> ![] {
    var maximum: isize = 0;
    var bit: usize = 0;
    let width = usize.width() - 1;
    while bit < width {
        maximum = maximum * 2 + 1;
        bit = bit + 1;
    }
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var negative = false;
    var minimum = 0 - maximum;
    var value: isize = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' {
                negative = true;
                minimum = 0 - maximum - 1;
                continue;
            }
        }
        let digit: isize = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value < minimum / 10 || (value == minimum / 10 && digit > 0 - (minimum % 10)) {
                overflow = true;
            } else {
                value = value * 10 - digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(if negative { value } else { 0 - value });
}

public flow from_u8(value: u8) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        let digit = remaining % 10;
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    return output;
}

public flow parse_u8(text: string) -> Result<u8, TextError> ![] {
    let maximum: u8 = 255;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var value: u8 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' { return Err(ParseFailure); }
        }
        let digit: u8 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value > maximum / 10 || (value == maximum / 10 && digit > maximum % 10) {
                overflow = true;
            } else {
                value = value * 10 + digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(value);
}

public flow from_u16(value: u16) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        let digit = remaining % 10;
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    return output;
}

public flow parse_u16(text: string) -> Result<u16, TextError> ![] {
    let maximum: u16 = 65535;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var value: u16 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' { return Err(ParseFailure); }
        }
        let digit: u16 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value > maximum / 10 || (value == maximum / 10 && digit > maximum % 10) {
                overflow = true;
            } else {
                value = value * 10 + digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(value);
}

public flow from_u32(value: u32) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        let digit = remaining % 10;
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    return output;
}

public flow parse_u32(text: string) -> Result<u32, TextError> ![] {
    let maximum: u32 = 4294967295;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var value: u32 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' { return Err(ParseFailure); }
        }
        let digit: u32 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value > maximum / 10 || (value == maximum / 10 && digit > maximum % 10) {
                overflow = true;
            } else {
                value = value * 10 + digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(value);
}

public flow from_u64(value: u64) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        let digit = remaining % 10;
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    return output;
}

public flow parse_u64(text: string) -> Result<u64, TextError> ![] {
    let maximum: u64 = 18446744073709551615;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var value: u64 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' { return Err(ParseFailure); }
        }
        let digit: u64 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value > maximum / 10 || (value == maximum / 10 && digit > maximum % 10) {
                overflow = true;
            } else {
                value = value * 10 + digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(value);
}

public flow from_u128(value: u128) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        let digit = remaining % 10;
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    return output;
}

public flow parse_u128(text: string) -> Result<u128, TextError> ![] {
    let maximum: u128 = 340282366920938463463374607431768211455;
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var value: u128 = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' { return Err(ParseFailure); }
        }
        let digit: u128 = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value > maximum / 10 || (value == maximum / 10 && digit > maximum % 10) {
                overflow = true;
            } else {
                value = value * 10 + digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(value);
}

public flow from_usize(value: usize) -> string ![] {
    if value == 0 { return "0"; }
    var remaining = value;
    var output = "";
    while remaining != 0 {
        let digit = remaining % 10;
        output = DECIMAL_DIGITS.get(digit).unwrap() + output;
        remaining = remaining / 10;
    }
    return output;
}

public flow parse_usize(text: string) -> Result<usize, TextError> ![] {
    var maximum: usize = 0;
    var bit: usize = 0;
    let width = usize.width();
    while bit < width {
        maximum = maximum * 2 + 1;
        bit = bit + 1;
    }
    var first = true;
    var saw_digit = false;
    var overflow = false;
    var value: usize = 0;
    for scalar in chars(text) {
        if first {
            first = false;
            if scalar == '+' { continue; }
            if scalar == '-' { return Err(ParseFailure); }
        }
        let digit: usize = match scalar {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => { return Err(ParseFailure); },
        };
        if !overflow {
            if value > maximum / 10 || (value == maximum / 10 && digit > maximum % 10) {
                overflow = true;
            } else {
                value = value * 10 + digit;
            }
        }
        saw_digit = true;
    }
    if !saw_digit { return Err(ParseFailure); }
    if overflow { return Err(NumericOverflow); }
    return Ok(value);
}
