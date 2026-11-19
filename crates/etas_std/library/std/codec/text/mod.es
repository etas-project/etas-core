module std.codec.text;

public import std.codec.text.MalformedInput.{Strict, Replace};
public import std.codec.text.TextCodecError.InvalidUtf8;


public enum MalformedInput { Strict, Replace, }
public enum TextCodecError { InvalidUtf8, }

public flow utf8_encode(value: string) -> bytes ![] {
    return value.utf8;
}

public flow utf8_decode(value: bytes, malformed: MalformedInput) -> Result<string, TextCodecError> ![] {
    var position: usize = 0;
    var output = "";
    while position < value.length limit Iterations(18446744073709551615) {
        let start = position;
        let lead = value.get(position).unwrap().bits;
        var width: usize = 0;
        var scalar: u128 = 0;
        if lead < 128 {
            width = 1;
            scalar = lead;
        } else if lead >= 194 && lead <= 223 {
            width = 2;
            scalar = lead % 32;
        } else if lead >= 224 && lead <= 239 {
            width = 3;
            scalar = lead % 16;
        } else if lead >= 240 && lead <= 244 {
            width = 4;
            scalar = lead % 8;
        }
        var valid = width != 0;
        var consumed: usize = 1;
        var index: usize = 1;
        while valid && index < width limit Iterations(3) {
            if start + index >= value.length {
                consumed = value.length - start;
                valid = false;
                break;
            }
            let continuation = value.get(start + index).unwrap().bits;
            if continuation < 128 || continuation > 191 {
                valid = false;
                break;
            }
            if index == 1 && (
                (lead == 224 && continuation < 160) ||
                (lead == 237 && continuation > 159) ||
                (lead == 240 && continuation < 144) ||
                (lead == 244 && continuation > 143)
            ) {
                valid = false;
                break;
            }
            scalar = scalar * 64 + continuation % 64;
            consumed = consumed + 1;
            index = index + 1;
        }
        if valid {
            output = output + std.text.from_char(char { bits= scalar });
            position = start + width;
        } else {
            match malformed {
                Strict => { return Err(InvalidUtf8); },
                Replace => { output = output + "\u{fffd}"; },
            };
            position = start + consumed;
        }
    }
    return Ok(output);
}
