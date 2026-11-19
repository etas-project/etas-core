module std.text;

impl FloatLayout {
    flow candidate_matches(self, significand: u128, exponent: i128, expected: u128) -> bool ![] {
        return match self.from_decimal(BigNat.new(significand), exponent) {
            Ok(bits) => bits == expected,
            Err(_) => false,
        };
    }

    flow decimal_text(self, significand: u128, exponent: i128, negative: bool) -> string ![] {
        var value = significand;
        var power = exponent;
        while value % 10 == 0 limit Iterations(17) {
            value = value / 10;
            power = power + 1;
        }
        let digits = from_u128(value);
        var output = "";
        if power >= 0 {
            output = digits;
            var zeros: i128 = 0;
            while zeros < power limit Iterations(308) {
                output = output + "0";
                zeros = zeros + 1;
            }
        } else {
            let point = i128 { bits= digits.length.bits } + power;
            if point <= 0 {
                output = "0.";
                var zeros: i128 = 0;
                while zeros < 0 - point limit Iterations(324) {
                    output = output + "0";
                    zeros = zeros + 1;
                }
                output = output + digits;
            } else {
                var index: i128 = 0;
                for scalar in chars(digits) limit Iterations(17) {
                    if index == point { output = output + "."; }
                    output = output + from_char(scalar);
                    index = index + 1;
                }
            }
        }
        return if negative { "-" + output } else { output };
    }

    flow render(self, bits: u128) -> string ![] {
        let sign_mask = self.sign_mask();
        let negative = bits >= sign_mask;
        let magnitude = bits % sign_mask;
        let implicit_bit = FLOAT_POWERS.get(self.fraction).unwrap();
        let encoded_exponent = magnitude / implicit_bit;
        let fraction = magnitude % implicit_bit;
        let exponent_ceiling = FLOAT_POWERS.get(self.exponent_bits).unwrap() - 1;
        if encoded_exponent == exponent_ceiling {
            if fraction != 0 { return "NaN"; }
            return if negative { "-inf" } else { "inf" };
        }
        if magnitude == 0 { return if negative { "-0" } else { "0" }; }
        let coefficient = if encoded_exponent == 0 { fraction } else { fraction + implicit_bit };
        let fraction_bits = i128 { bits= self.fraction.bits };
        let binary_exponent = if encoded_exponent == 0 {
            1 - self.bias - fraction_bits
        } else {
            i128 { bits= encoded_exponent } - self.bias - fraction_bits
        };
        var exact = BigNat.new(coefficient);
        var decimal_places: i128 = 0;
        if binary_exponent >= 0 {
            exact = exact.shift_left(usize { bits= binary_exponent.bits });
        } else {
            decimal_places = 0 - binary_exponent;
            exact = exact.power_scale(5, usize { bits= decimal_places.bits });
        }
        let exact_digits = exact.decimal().utf8;
        let exponent = i128 { bits= exact_digits.length.bits } - decimal_places - 1;
        var prefix: u128 = 0;
        var precision: usize = 0;
        while precision < self.roundtrip_digits && precision < exact_digits.length
            limit Iterations(17) {
            prefix = prefix * 10 + exact_digits.get(precision).unwrap().bits - 48;
            precision = precision + 1;
            let power = exponent - i128 { bits= precision.bits } + 1;
            let floor_matches = self.candidate_matches(prefix, power, magnitude);
            let ceiling_matches = self.candidate_matches(prefix + 1, power, magnitude);
            if floor_matches || ceiling_matches {
                var round_up = !floor_matches;
                if floor_matches && ceiling_matches && precision < exact_digits.length {
                    let following = exact_digits.get(precision).unwrap();
                    round_up = following > 53;
                    if following == 53 {
                        round_up = prefix % 2 == 1;
                        var index = precision + 1;
                        while index < exact_digits.length limit Iterations(1075) {
                            if exact_digits.get(index) != Some(48) {
                                round_up = true;
                                break;
                            }
                            index = index + 1;
                        }
                    }
                }
                let selected = if round_up { prefix + 1 } else { prefix };
                return self.decimal_text(selected, power, negative);
            }
        }
        std.core.abort("finite float has no roundtrip decimal within max_digits10");
    }
}

public flow from_f32(value: f32) -> string ![] {
    let bits = value.bits;
    return FLOAT32.render(bits);
}

public flow from_f64(value: f64) -> string ![] {
    let bits = value.bits;
    return FLOAT64.render(bits);
}
