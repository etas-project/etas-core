module std.math;

impl FloatEncoding {
    // floor((2/pi) * 2^1216), in little-endian 32-bit limbs. The
    // reciprocal error contributes less than 2^-192 for any finite f64.
    flow two_over_pi() -> Array<u128> ![] {
        return [
            0xfc7b6bab, 0x56033046, 0x1f8d5d08, 0x6bfb5fb1,
            0x8a5292ea, 0x3d0739f7, 0xebe5f17b, 0x7527bac7,
            0x9e5fea2d, 0x4f463f66, 0x27cb09b7, 0x6d367ecf,
            0x5a0a6d1f, 0xef2f118b, 0xde05980f, 0x1ff897ff,
            0xbdf9283b, 0x9c845f8b, 0x835339f4, 0x3991d639,
            0xb45f7e41, 0xe99c7026, 0x2ebb4484, 0xe88235f5,
            0xb129a73e, 0xfe1deb1c, 0x09d1921c, 0x06492eea,
            0x424dd2e0, 0xb7246e3a, 0xdebbc561, 0xfe5163ab,
            0x3c439041, 0xdb629599, 0xf534ddc0, 0xfc2757d1,
            0x4e441529, 0xa2f9836e,
        ];
    }

    flow limb_power(count: usize) -> u128 ![] {
        var result: u128 = 1;
        var index: usize = 0;
        while index < count {
            result = result * 2;
            index = index + 1;
        }
        return result;
    }

    flow limb_bit(words: Array<u128>, position: usize) -> u128 ![] {
        let word = words.get(position / 32).unwrap_or(0);
        return (word / FloatEncoding.limb_power(position % 32)) % 2;
    }

    // Convert an exact fixed-point fraction, rounding its significand once.
    flow fraction<F ~ Float>(self, words: Array<u128>, shift: usize) -> F ![] {
        var leading = shift;
        while leading > 0 {
            if FloatEncoding.limb_bit(words, leading - 1) != 0 { break; }
            leading = leading - 1;
        }
        if leading == 0 { return F { bits= 0 }; }
        var exponent = i32 { bits= (leading - 1).bits } - i32 { bits= shift.bits };
        var significand: u128 = 1;
        var cursor = leading - 1;
        var count: u128 = 0;
        while count < self.fraction_width {
            significand = significand * 2;
            if cursor > 0 {
                cursor = cursor - 1;
                significand = significand + FloatEncoding.limb_bit(words, cursor);
            }
            count = count + 1;
        }
        let guard = if cursor == 0 { 0 } else { FloatEncoding.limb_bit(words, cursor - 1) };
        var sticky = false;
        if cursor > 1 {
            var remaining = cursor - 1;
            while remaining > 0 {
                remaining = remaining - 1;
                if FloatEncoding.limb_bit(words, remaining) != 0 { sticky = true; break; }
            }
        }
        if guard != 0 && (sticky || significand % 2 != 0) { significand = significand + 1; }
        if significand == self.scale * 2 {
            significand = self.scale;
            exponent = exponent + 1;
        }
        let normalized = F { bits= self.bias * self.scale + significand - self.scale };
        return self.scaled(normalized, exponent);
    }

    // Payne-Hanek reduction: the large quotient stays in integer limbs.
    // Only its quadrant and a signed fraction are converted to F.
    flow reduced_angle<F ~ Float>(self, magnitude: F) -> (usize, F) ![] {
        if magnitude <= self.constant<F>(0x3f490fdb, 0x3fe921fb54442d18) {
            return (0, magnitude);
        }
        let raw_exponent = magnitude.bits / self.scale;
        let exponent = i32 { bits= raw_exponent } - i32 { bits= self.bias };
        let significand = self.scale + magnitude.bits % self.scale;
        let shift = usize { bits= (1216 + i32 { bits= self.fraction_width } - exponent).bits };
        let base: u128 = 4294967296;
        var product: Array<u128> = [];
        var carry: u128 = 0;
        for digit in FloatEncoding.two_over_pi() {
            let multiplied = digit * significand + carry;
            product = product.push(multiplied % base);
            carry = multiplied / base;
        }
        while carry != 0 {
            product = product.push(carry % base);
            carry = carry / base;
        }
        let round_up = FloatEncoding.limb_bit(product, shift - 1) != 0;
        let quadrant = (FloatEncoding.limb_bit(product, shift) + 2 * FloatEncoding.limb_bit(product, shift + 1) + if round_up { 1 } else { 0 }) % 4;
        let count = (shift + 31) / 32;
        var words: Array<u128> = [];
        var index: usize = 0;
        var complement_carry = true;
        while index < count {
            let original = product.get(index).unwrap_or(0);
            var word = original;
            if round_up {
                if complement_carry {
                    if original == 0 { word = 0; }
                    else { word = base - original; complement_carry = false; }
                } else { word = base - 1 - original; }
            }
            if index + 1 == count && shift % 32 != 0 {
                word = word % FloatEncoding.limb_power(shift % 32);
            }
            words = words.push(word);
            index = index + 1;
        }
        let fraction: F = self.fraction(words, shift);
        let signed = if round_up { -fraction } else { fraction };
        let half_pi_high = self.constant<F>(0x3fc90000, 0x3ff921fb54400000);
        let half_pi_low = self.constant<F>(0x39fdaa22, 0x3dd0b4611a626331);
        return (usize { bits= quadrant }, signed * half_pi_high + signed * half_pi_low);
    }

    flow sine_cosine<F ~ Float>(self, reduced: F) -> (F, F) ![] {
        let square = -(reduced * reduced);
        var sine = reduced;
        var cosine: F = 1;
        var sine_term = reduced;
        var cosine_term: F = 1;
        var divisor: F = 2;
        var count: usize = 0;
        while count < 11 {
            sine_term = sine_term * square / (divisor * (divisor + 1));
            cosine_term = cosine_term * square / (divisor * (divisor - 1));
            sine = sine + sine_term;
            cosine = cosine + cosine_term;
            divisor = divisor + 2;
            count = count + 1;
        }
        return (sine, cosine);
    }
}
