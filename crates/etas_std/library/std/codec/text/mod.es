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
    // Materialize bounded batches so long inputs do not retain an array of every scalar.
    var scalars: Array<char> = [];
    var text = "";
    while position < value.length {
        if scalars.length == 64 {
            text = text + string { scalars= scalars };
            scalars = [];
        }
        let lead = value[position]?.unwrap().bits;
        position = position + 1;
        if lead < 128 {
            scalars = scalars + [char { bits= lead }];
            continue;
        }
        var first_low: u128 = 128;
        var first_high: u128 = 191;
        var scalar: u128 = 0;
        var remaining: usize = 0;
        if lead >= 194 && lead <= 223 {
            scalar = lead - 192;
            remaining = 1;
        } else if lead >= 224 && lead <= 239 {
            scalar = lead - 224;
            remaining = 2;
            if lead == 224 { first_low = 160; }
            if lead == 237 { first_high = 159; }
        } else if lead >= 240 && lead <= 244 {
            scalar = lead - 240;
            remaining = 3;
            if lead == 240 { first_low = 144; }
            if lead == 244 { first_high = 143; }
        }
        if remaining != 0 && position < value.length {
            let second = value[position]?.unwrap().bits;
            if second >= first_low && second <= first_high {
                scalar = scalar * 64 + second - 128;
                position = position + 1;
                remaining = remaining - 1;
                while remaining != 0 && position < value.length {
                    let byte = value[position]?.unwrap().bits;
                    if byte < 128 || byte > 191 { break; }
                    scalar = scalar * 64 + byte - 128;
                    position = position + 1;
                    remaining = remaining - 1;
                }
                if remaining == 0 {
                    scalars = scalars + [char { bits= scalar }];
                    continue;
                }
            }
        }
        if malformed == Strict { return Err(InvalidUtf8); }
        scalars = scalars + ['�'];
    }
    return Ok(text + string { scalars= scalars });
}
