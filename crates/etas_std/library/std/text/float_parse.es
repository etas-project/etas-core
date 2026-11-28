module std.text;

import std.text.TextError.{NumericOverflow, ParseFailure};

type FloatLayout = {
    private fraction: usize,
    private exponent_bits: usize,
    private bias: i128,
    private minimum_exponent: i128,
    private maximum_exponent: i128,
    private minimum_decimal: i128,
    private maximum_decimal: i128,
};

let FLOAT32 = FloatLayout {
    fraction= 23, exponent_bits= 8, bias= 127,
    minimum_exponent= -126, maximum_exponent= 127,
    minimum_decimal= -46, maximum_decimal= 38,
};

let FLOAT64 = FloatLayout {
    fraction= 52, exponent_bits= 11, bias= 1023,
    minimum_exponent= -1022, maximum_exponent= 1023,
    minimum_decimal= -324, maximum_decimal= 308,
};

impl FloatLayout {
    flow sign_mask(self) -> u128 ![] {
        return FLOAT_POWERS.get(self.fraction + self.exponent_bits).unwrap();
    }

    flow parse(self, text: string) -> Result<u128, TextError> ![] {
        let input = text.utf8;
        var position: usize = 0;
        var negative = false;
        if input.get(0) == Some(45) || input.get(0) == Some(43) {
            negative = input.get(0) == Some(45);
            position = 1;
        }
        var number: u128 = 0;
        // Binary64 rounding midpoints have at most 769 significant decimal digits
        // (a 54-bit integer times at most 5^1075). Binary32 needs fewer.
        let precision: usize = 800;
        var sticky = false;
        var digits: usize = 0;
        var fractional: usize = 0;
        var significant: usize = 0;
        var last_nonzero: usize = 0;
        var decimal = false;
        var exponent = false;
        while position < input.length {
            let byte = input.get(position).unwrap();
            if byte >= 48 && byte <= 57 {
                let digit = byte.bits - 48;
                if significant < 19 { number = number * 10 + digit; }
                if significant >= precision && digit != 0 { sticky = true; }
                digits = digits + 1;
                if decimal { fractional = fractional + 1; }
                if byte != 48 || significant != 0 { significant = significant + 1; }
                if byte != 48 { last_nonzero = significant; }
                position = position + 1;
            } else if byte == 46 && !decimal {
                decimal = true;
                position = position + 1;
            } else if (byte == 101 || byte == 69) && digits != 0 {
                exponent = true;
                position = position + 1;
                break;
            } else {
                return Err(ParseFailure);
            }
        }
        if digits == 0 { return Err(ParseFailure); }
        var exponent_negative = false;
        var magnitude: u128 = 0;
        if exponent {
            if input.get(position) == Some(45) || input.get(position) == Some(43) {
                exponent_negative = input.get(position) == Some(45);
                position = position + 1;
            }
            var exponent_digits: usize = 0;
            // Beyond input length + 4096, no decimal point or leading zeros
            // in this same input can bring a nonzero value into either format.
            let cap = input.length.bits + 4096;
            while position < input.length {
                let byte = input.get(position).unwrap();
                if byte < 48 || byte > 57 { return Err(ParseFailure); }
                if magnitude < cap {
                    magnitude = magnitude * 10 + byte.bits - 48;
                } else {
                    magnitude = cap;
                }
                position = position + 1;
                exponent_digits = exponent_digits + 1;
            }
            if exponent_digits == 0 { return Err(ParseFailure); }
        }
        let sign = if negative { self.sign_mask() } else { 0 };
        if significant == 0 { return Ok(sign); }
        let absolute_exponent = i128 { bits= magnitude };
        let explicit_exponent = if exponent_negative { 0 - absolute_exponent } else { absolute_exponent };
        var power = explicit_exponent - i128 { bits= fractional.bits };
        let decimal_exponent = i128 { bits= significant.bits } - 1 + power;
        if decimal_exponent < self.minimum_decimal || decimal_exponent > self.maximum_decimal {
            return Err(NumericOverflow);
        }
        if significant > 19 && last_nonzero < significant {
            power = power + i128 { bits= (significant - last_nonzero).bits };
            significant = last_nonzero;
            var used: usize = 19;
            while used > significant { number = number / 10; used = used - 1; }
        }
        if significant <= 19 {
            match self.fast_decimal(number, power) {
                Some(bits) => { return Ok(sign + bits); },
                None => { return self.from_decimal(BigNat.new(number), power).map((bits: u128) => sign + bits); },
            };
        }
        let retained = if significant < precision { significant } else { precision };
        var exact = BigNat.new(0);
        var count: usize = 0;
        var chunk: u128 = 0;
        var scale: u128 = 1;
        for byte in input {
            if byte < 48 || byte > 57 { continue; }
            let digit = byte.bits - 48;
            if count == 0 && digit == 0 { continue; }
            chunk = chunk * 10 + digit;
            scale = scale * 10;
            count = count + 1;
            if scale == 1000000000 || count == retained {
                exact = exact.multiply_small(scale).add_small(chunk);
                chunk = 0;
                scale = 1;
            }
            if count == retained { break; }
        }
        var adjusted = power + i128 { bits= (significant - retained).bits };
        if sticky {
            // All nonzero discarded tails lie on the same side of every midpoint.
            exact = exact.multiply_small(10).add_small(1);
            adjusted = adjusted - 1;
        }
        return self.from_decimal(exact, adjusted).map((bits: u128) => sign + bits);
    }

    // Eisel-Lemire conversion adapted from Go 1.23 strconv/eisel_lemire.go.
    // Copyright 2020 The Go Authors. See licenses/GO-BSD-3-Clause.txt.
    // Ambiguous rounding, subnormals and overflow retain the exact BigNat path.
    flow fast_decimal(self, number: u128, exponent: i128) -> Option<u128> ![] {
        if number == 0 { return Some(0); }
        if exponent < -348 || exponent > 347 { return None; }
        var mantissa = number;
        var leading: i128 = 0;
        if mantissa < 4294967296 { mantissa = mantissa * 4294967296; leading = leading + 32; }
        if mantissa < 281474976710656 { mantissa = mantissa * 65536; leading = leading + 16; }
        if mantissa < 72057594037927936 { mantissa = mantissa * 256; leading = leading + 8; }
        if mantissa < 1152921504606846976 { mantissa = mantissa * 16; leading = leading + 4; }
        if mantissa < 4611686018427387904 { mantissa = mantissa * 4; leading = leading + 2; }
        if mantissa < 9223372036854775808 { mantissa = mantissa * 2; leading = leading + 1; }
        var binary_exponent = self.floor_ratio(217706 * exponent, 65536) + 64 + self.bias - leading;
        let factor = DECIMAL_POWERS.get(usize { bits= (exponent + 348).bits }).unwrap();
        let product = mantissa * factor.high;
        var high = product / 18446744073709551616;
        var low = product % 18446744073709551616;
        let shift: usize = 61 - self.fraction;
        let unit = FLOAT_POWERS.get(shift).unwrap();
        if high % unit == unit - 1 && low + mantissa >= 18446744073709551616 {
            let wider = mantissa * factor.low;
            let middle = low + wider / 18446744073709551616;
            high = high + middle / 18446744073709551616;
            low = middle % 18446744073709551616;
            if high % unit == unit - 1 && low == 18446744073709551615
                && wider % 18446744073709551616 + mantissa >= 18446744073709551616 { return None; }
        }
        let msb = high / 9223372036854775808;
        var rounded = high / FLOAT_POWERS.get(shift + usize { bits= msb }).unwrap();
        binary_exponent = binary_exponent - i128 { bits= 1 - msb };
        if low == 0 && high % unit == 0 && rounded % 4 == 1 { return None; }
        rounded = (rounded + rounded % 2) / 2;
        if rounded >= FLOAT_POWERS.get(self.fraction + 1).unwrap() {
            rounded = rounded / 2;
            binary_exponent = binary_exponent + 1;
        }
        let ceiling = i128 { bits= FLOAT_POWERS.get(self.exponent_bits).unwrap() - 1 };
        if binary_exponent <= 0 || binary_exponent >= ceiling { return None; }
        let implicit = FLOAT_POWERS.get(self.fraction).unwrap();
        return Some(binary_exponent.bits * implicit + rounded % implicit);
    }

    flow from_decimal(self, significand: BigNat, exponent: i128) -> Result<u128, TextError> ![] {
        if significand.is_zero() { return Ok(0); }
        var numerator = significand;
        var denominator = BigNat.new(1);
        if exponent >= 0 {
            numerator = numerator.power_scale(10, usize { bits= exponent.bits });
        } else {
            let magnitude = 0 - exponent;
            denominator = denominator.power_scale(10, usize { bits= magnitude.bits });
        }
        let numerator_bits = numerator.bit_length();
        let denominator_bits = denominator.bit_length();
        var binary_exponent = i128 { bits= numerator_bits.bits } - i128 { bits= denominator_bits.bits };
        if binary_exponent >= 0 {
            if numerator.compare(denominator.shift_left(usize { bits= binary_exponent.bits })) < 0 {
                binary_exponent = binary_exponent - 1;
            }
        } else {
            let magnitude = 0 - binary_exponent;
            if numerator.shift_left(usize { bits= magnitude.bits }).compare(denominator) < 0 {
                binary_exponent = binary_exponent - 1;
            }
        }
        let fraction_bits = i128 { bits= self.fraction.bits };
        let minimum_unit = self.minimum_exponent - fraction_bits;
        var unit = binary_exponent - fraction_bits;
        if unit < minimum_unit { unit = minimum_unit; }
        if unit < 0 {
            let magnitude = 0 - unit;
            numerator = numerator.shift_left(usize { bits= magnitude.bits });
        } else {
            denominator = denominator.shift_left(usize { bits= unit.bits });
        }
        let (quotient, remainder) = numerator.divide_significand(denominator);
        var rounded = quotient;
        let halfway = remainder.multiply_small(2).compare(denominator);
        if halfway > 0 || (halfway == 0 && rounded % 2 == 1) {
            rounded = rounded + 1;
        }
        if rounded == 0 { return Err(NumericOverflow); }
        let implicit_bit = FLOAT_POWERS.get(self.fraction).unwrap();
        if rounded >= implicit_bit * 2 {
            rounded = rounded / 2;
            unit = unit + 1;
        }
        if unit == minimum_unit && rounded < implicit_bit { return Ok(rounded); }
        let encoded_exponent = unit + fraction_bits;
        if encoded_exponent > self.maximum_exponent { return Err(NumericOverflow); }
        let biased = encoded_exponent + self.bias;
        return Ok(biased.bits * implicit_bit + rounded - implicit_bit);
    }
}

public flow parse_f32(text: string) -> Result<f32, TextError> ![] {
    return match FLOAT32.parse(text) {
        Ok(bits) => Ok(f32 { bits= bits }),
        Err(error) => Err(error),
    };
}

public flow parse_f64(text: string) -> Result<f64, TextError> ![] {
    return match FLOAT64.parse(text) {
        Ok(bits) => Ok(f64 { bits= bits }),
        Err(error) => Err(error),
    };
}
