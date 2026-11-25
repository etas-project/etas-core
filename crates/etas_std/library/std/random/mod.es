module std.random;

public import std.effects.Random;

public enum RandomError { InvalidRange, Unavailable, Host(string) }

// SplitMix64, profile v1. Copies retain an independent sequence position.
public type Rng = { private state: u64 };

impl Rng {
    public flow new(seed: u64) -> Rng ![] {
        return Rng { state= seed };
    }

    flow xor(left: u128, right: u128) -> u128 ![] {
        var a = left;
        var b = right;
        var result: u128 = 0;
        var place: u128 = 1;
        while a != 0 || b != 0 {
            result = result + ((a % 2 + b % 2) % 2) * place;
            a = a / 2;
            b = b / 2;
            place = place * 2;
        }
        return result;
    }

    public flow next_u64(self) -> (Rng, u64) ![] {
        let state = (self.state.bits + 11400714819323198485) % 18446744073709551616;
        let first = (Rng.xor(state, state / 1073741824) * 13787848793156543929) % 18446744073709551616;
        let second = (Rng.xor(first, first / 134217728) * 10723151780598845931) % 18446744073709551616;
        let output = Rng.xor(second, second / 2147483648);
        return (Rng { state= u64 { bits= state } }, u64 { bits= output });
    }

    public flow next_f64(self) -> (Rng, f64) ![] {
        let (next, word) = self.next_u64();
        var significand = word.bits / 2048;
        if significand == 0 { return (next, 0.0); }
        var exponent: i32 = -1;
        while significand < 4503599627370496 {
            significand = significand * 2;
            exponent = exponent - 1;
        }
        let biased = u128 { bits= (exponent + 1023).bits };
        return (next, f64 { bits= biased * 4503599627370496 + significand - 4503599627370496 });
    }

    public flow int_range<I ~ std.math.Integer>(self, lower: I, upper: I) -> Result<(Rng, I), RandomError> ![] {
        if lower >= upper { return Err(RandomError.InvalidRange); }
        let all = (I { bits= 340282366920938463463374607431768211455 });
        let signed = all < (I { bits= 0 });
        let sign = all.bits / 2 + 1;
        let low = if signed {
            if lower.bits < sign { lower.bits + sign } else { lower.bits - sign }
        } else { lower.bits };
        let high = if signed {
            if upper.bits < sign { upper.bits + sign } else { upper.bits - sign }
        } else { upper.bits };
        let span = high - low;
        let threshold = (all.bits % span + 1) % span;
        var current = self;
        while true {
            let (next, word) = current.next_u64();
            current = next;
            var draw = word.bits;
            if all.bits > 18446744073709551615 {
                let (after, second) = current.next_u64();
                current = after;
                draw = draw * 18446744073709551616 + second.bits;
            }
            draw = (I { bits= draw }).bits;
            if draw >= threshold {
                let ordered = low + draw % span;
                let bits = if signed {
                    if ordered < sign { ordered + sign } else { ordered - sign }
                } else { ordered };
                return Ok((current, I { bits= bits }));
            }
        }
    }

    public flow choose<T>(self, values: Array<T>) -> (Rng, Option<T>) ![] {
        if values.is_empty() { return (self, None); }
        let (next, index) = self.int_range<usize>(0, values.length).unwrap();
        return (next, values.get(index));
    }

    public flow shuffle<T>(self, values: Array<T>) -> (Rng, Array<T>) ![] {
        var current = self;
        var output = values;
        var remaining = values.length;
        while remaining > 1 {
            let (next, index) = current.int_range<usize>(0, remaining).unwrap();
            current = next;
            remaining = remaining - 1;
            let chosen = output.get(index).unwrap();
            let last = output.get(remaining).unwrap();
            output = output.set(index, last)?.unwrap();
            output = output.set(remaining, chosen)?.unwrap();
        }
        return (current, output);
    }
}

// Empty output does not observe the host or advance a caller's entropy handler.
public flow secure_bytes(count: usize) -> bytes ![Error<RandomError>] {
    if count == 0 {
        return bytes { data= [] };
    }
    return perform Random.entropy(count);
}
