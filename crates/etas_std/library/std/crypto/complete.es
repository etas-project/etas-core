module std.crypto;

public type CryptoError = { code: string, message: string };
public alias Digest = bytes;

public flow constant_time_eq(left: bytes, right: bytes) -> bool ![] {
    let length = if left.length > right.length { left.length } else { right.length };
    var difference: u128 = if left.length == right.length { 0 } else { 1 };
    var index: usize = 0;
    while index < length {
        let a: u8 = match left.get(index) { Some(value) => value, None => 0 };
        let b: u8 = match right.get(index) { Some(value) => value, None => 0 };
        let delta = i32 { bits= a.bits } - i32 { bits= b.bits };
        let square = delta * delta;
        difference = difference + square.bits;
        index = index + 1;
    }
    return difference == 0;
}

// Host-secret handles may be non-exportable. This is the separately declared
// Secret.use operation required by the host architecture, not the public hash
// algorithm's execution path.
public flow hmac_sha256<K>(key: std.secret.SecretValue<K>, body: bytes)
    -> Digest ![Error<CryptoError>] {
    return perform std.effects.Secret.use<K>(key, body);
}

public flow sha256(input: bytes) -> Digest ![] {
    var state = Sha256State.new();
    let prefix = input.length - input.length % 64;
    var offset: usize = 0;
    while offset < prefix {
        state = state.compress(input, offset);
        offset = offset + 64;
    }

    var padding: Array<u8> = [];
    while offset < input.length {
        padding = padding.push(input.get(offset).unwrap());
        offset = offset + 1;
    }
    padding = padding.push(128);
    while padding.length % 64 != 56 {
        padding = padding.push(0);
    }
    let bit_length = (input.length.bits * 8) % 18446744073709551616;
    var length_byte: usize = 0;
    while length_byte < 8 {
        let divisor = BYTE_POWERS.get(7 - length_byte).unwrap();
        padding = padding.push(u8 { bits= (bit_length / divisor) % 256 });
        length_byte = length_byte + 1;
    }
    let tail = bytes { data= padding };
    offset = 0;
    while offset < tail.length {
        state = state.compress(tail, offset);
        offset = offset + 64;
    }

    var output: Array<u8> = [];
    for word in state.words {
        output = output.extend([
            u8 { bits= word.raw / 16777216 },
            u8 { bits= (word.raw / 65536) % 256 },
            u8 { bits= (word.raw / 256) % 256 },
            u8 { bits= word.raw % 256 },
        ]);
    }
    return bytes { data= output };
}
