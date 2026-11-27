module std.text;

impl FloatLayout {
    // Ryu interval arithmetic adapted from Go 1.23 strconv/ftoaryu.go.
    // Copyright 2021 The Go Authors. See licenses/GO-BSD-3-Clause.txt.
    flow floor_ratio(self, numerator: i128, denominator: i128) -> i128 ![] {
        let quotient = numerator / denominator;
        return if numerator < 0 && numerator % denominator != 0 { quotient - 1 } else { quotient };
    }
    flow divisible_power5(self, value: u128, power: usize) -> bool ![] {
        var number = value;
        var remaining = power;
        while remaining > 0 {
            if number % 5 != 0 { return false; }
            number = number / 5;
            remaining = remaining - 1;
        }
        return true;
    }
    flow scale_decimal(self, value: u128, exponent: i128, power: i128) -> (u128, i128, bool) ![] {
        if power == 0 { return (value * 256, exponent - 8, true); }
        let factor = DECIMAL_POWERS.get(usize { bits= (power + 348).bits }).unwrap();
        let low = factor.low + if power < 0 { 1 } else { 0 };
        let bottom = value * low;
        let top = value * factor.high;
        let middle = bottom / 18446744073709551616 + top % 18446744073709551616;
        let high = top / 18446744073709551616 + middle / 18446744073709551616;
        let result = high * 512 + (middle % 18446744073709551616) / 36028797018963968;
        let exact = middle % 36028797018963968 == 0 && bottom % 18446744073709551616 == 0;
        return (result, exponent + self.floor_ratio(power * 108853, 32768) - 8, exact);
    }
    flow shortest_digits(self, lower: u128, central: u128, upper: u128, zero: bool, round_up: bool) -> (u128, i128) ![] {
        var low = lower; var middle = central; var high = upper;
        var trailing_zero = zero; var up = round_up;
        var next_digit: u128 = 0;
        var removed: i128 = 0;
        while high > 0 {
            let next_low = (low + 9) / 10;
            var next_middle = middle / 10;
            var digit = middle % 10;
            let next_high = high / 10;
            if next_low > next_high { break; }
            if next_low == next_middle + 1 && next_middle < next_high {
                next_middle = next_middle + 1;
                digit = 0;
                up = false;
            }
            trailing_zero = trailing_zero && next_digit == 0;
            next_digit = digit;
            removed = removed + 1;
            low = next_low; middle = next_middle; high = next_high;
        }
        if removed > 0 {
            up = next_digit > 5 || (next_digit == 5 && (!trailing_zero || middle % 2 == 1));
        }
        if middle < high && up { middle = middle + 1; }
        return (middle, removed);
    }

    flow decimal_text(self, significand: u128, exponent: i128, negative: bool) -> string ![] {
        var value = significand;
        var power = exponent;
        while value % 10 == 0 {
            value = value / 10;
            power = power + 1;
        }
        let digits = from_u128(value);
        var output = "";
        if power >= 0 {
            output = digits;
            var zeros: i128 = 0;
            while zeros < power {
                output = output + "0";
                zeros = zeros + 1;
            }
        } else {
            let point = i128 { bits= digits.length.bits } + power;
            if point <= 0 {
                output = "0.";
                var zeros: i128 = 0;
                while zeros < 0 - point {
                    output = output + "0";
                    zeros = zeros + 1;
                }
                output = output + digits;
            } else {
                var index: i128 = 0;
                for scalar in chars(digits) {
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
        // Ryu: scale the rounding interval once, then remove decimal digits
        // while at least one representation remains inside that interval.
        if binary_exponent <= 0 && binary_exponent >= -63 {
            let divisor = FLOAT_POWERS.get(usize { bits= (0 - binary_exponent).bits }).unwrap();
            if coefficient % divisor == 0 {
                return self.decimal_text(coefficient / divisor, 0, negative);
            }
        }
        let regular = coefficient != implicit_bit || binary_exponent == self.minimum_exponent - fraction_bits;
        var lower = if regular { 2 * coefficient - 1 } else { 4 * coefficient - 1 };
        var central = if regular { 2 * coefficient } else { 4 * coefficient };
        var upper = if regular { 2 * coefficient + 1 } else { 4 * coefficient + 2 };
        let exponent2 = binary_exponent - if regular { 1 } else { 2 };
        if exponent2 == 0 {
            let (digits, removed) = self.shortest_digits(lower, central, upper, true, false);
            return self.decimal_text(digits, removed, negative);
        }
        let q = self.floor_ratio((0 - exponent2) * 78913, 262144) + 1;
        let (scaled_lower, _, exact_lower) = self.scale_decimal(lower, exponent2, q);
        let (scaled_central, _, exact_central) = self.scale_decimal(central, exponent2, q);
        let (scaled_upper, scaled_exponent, exact_upper) = self.scale_decimal(upper, exponent2, q);
        var lower_zero = exact_lower && q <= 55;
        var central_zero = exact_central && q <= 55;
        var upper_zero = exact_upper && q <= 55;
        if q < 0 && q >= -24 {
            let power = usize { bits= (0 - q).bits };
            lower_zero = lower_zero || self.divisible_power5(lower, power);
            central_zero = central_zero || self.divisible_power5(central, power);
            upper_zero = upper_zero || self.divisible_power5(upper, power);
        }
        let divisor = FLOAT_POWERS.get(usize { bits= (0 - scaled_exponent).bits }).unwrap();
        let lower_fraction = scaled_lower % divisor;
        let central_fraction = scaled_central % divisor;
        let upper_fraction = scaled_upper % divisor;
        lower = scaled_lower / divisor;
        central = scaled_central / divisor;
        upper = scaled_upper / divisor;
        if upper_zero && upper_fraction == 0 && coefficient % 2 != 0 { upper = upper - 1; }
        let round_up = if central_zero {
            central_fraction > divisor / 2 || (central_fraction == divisor / 2 && central % 2 == 1)
        } else { central_fraction >= divisor / 2 };
        if !lower_zero || lower_fraction != 0 || coefficient % 2 != 0 { lower = lower + 1; }
        let (digits, removed) = self.shortest_digits(lower, central, upper, central_zero && central_fraction == 0, round_up);
        return self.decimal_text(digits, removed - q, negative);
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
