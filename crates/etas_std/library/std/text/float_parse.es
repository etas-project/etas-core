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
    private roundtrip_digits: usize,
};

let FLOAT32 = FloatLayout {
    fraction= 23, exponent_bits= 8, bias= 127,
    minimum_exponent= -126, maximum_exponent= 127,
    minimum_decimal= -46, maximum_decimal= 38, roundtrip_digits= 9,
};

let FLOAT64 = FloatLayout {
    fraction= 52, exponent_bits= 11, bias= 1023,
    minimum_exponent= -1022, maximum_exponent= 1023,
    minimum_decimal= -324, maximum_decimal= 308, roundtrip_digits= 17,
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
        var number = BigNat.new(0);
        var digits: usize = 0;
        var fractional: usize = 0;
        var significant: usize = 0;
        var nonzero = false;
        var decimal = false;
        var exponent = false;
        while position < input.length {
            let byte = input.get(position).unwrap();
            if byte >= 48 && byte <= 57 {
                number = number.multiply_small(10).add_small(byte.bits - 48);
                digits = digits + 1;
                if decimal { fractional = fractional + 1; }
                if byte != 48 { nonzero = true; }
                if nonzero { significant = significant + 1; }
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
        if !nonzero { return Ok(sign); }
        let absolute_exponent = i128 { bits= magnitude };
        let explicit_exponent = if exponent_negative { 0 - absolute_exponent } else { absolute_exponent };
        let power = explicit_exponent - i128 { bits= fractional.bits };
        let decimal_exponent = i128 { bits= significant.bits } - 1 + power;
        if decimal_exponent < self.minimum_decimal || decimal_exponent > self.maximum_decimal {
            return Err(NumericOverflow);
        }
        return match self.from_decimal(number, power) {
            Ok(bits) => Ok(sign + bits),
            Err(error) => Err(error),
        };
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
