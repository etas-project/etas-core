module std.math;

impl FloatEncoding {
    flow constant<F ~ Float>(self, narrow: u128, wide: u128) -> F ![] {
        return F { bits= if self.fraction_width == 23 { narrow } else { wide } };
    }

    // Exact conversion for the small exponents used during range reduction.
    flow integer<F ~ Float>(self, value: i32) -> F ![] {
        if value == 0 { return F { bits= 0 }; }
        let negative = value < 0;
        var significand = u128 { bits= (if negative { -value } else { value }).bits };
        var exponent = i32 { bits= self.fraction_width };
        while significand < self.scale {
            significand = significand * 2;
            exponent = exponent - 1;
        }
        let biased = u128 { bits= (exponent + i32 { bits= self.bias }).bits };
        return F { bits= (if negative { self.sign } else { 0 }) + biased * self.scale + significand - self.scale };
    }

    flow small_integer(self) -> i32 ![] {
        let magnitude = self.bits % self.sign;
        let exponent = magnitude / self.scale;
        if exponent < self.bias { return 0; }
        var remaining = exponent - self.bias;
        var significand = self.scale + magnitude % self.scale;
        while remaining > 0 {
            significand = significand * 2;
            remaining = remaining - 1;
        }
        let value = i32 { bits= significand / self.scale };
        return if self.bits >= self.sign { -value } else { value };
    }

    // Above the last significand bit every representable integer is even.
    flow integer_parity(self) -> Option<bool> ![] {
        let magnitude = self.bits % self.sign;
        if magnitude == 0 { return Some(false); }
        let exponent = magnitude / self.scale;
        if exponent < self.bias { return None; }
        if exponent > self.bias + self.fraction_width { return Some(false); }
        let unit = self.unit();
        if magnitude % unit != 0 { return None; }
        return Some((magnitude / unit) % 2 != 0);
    }

    // Adjust the exponent without an overflowing intermediate power of two.
    // The subnormal path rounds the exact integer significand, ties to even.
    flow scaled<F ~ Float>(self, value: F, shift: i32) -> F ![] {
        let magnitude = value.bits % self.sign;
        let exponent = i32 { bits= magnitude / self.scale } + shift;
        if exponent >= (i32 { bits= self.exponent_maximum }) {
            return F { bits= self.exponent_maximum * self.scale };
        }
        if exponent > 0 {
            return F { bits= u128 { bits= exponent.bits } * self.scale + magnitude % self.scale };
        }
        let remaining = u128 { bits= (1 - exponent).bits };
        if remaining > self.fraction_width + 1 { return F { bits= 0 }; }
        var divisor: u128 = 1;
        var count: u128 = 0;
        while count < remaining {
            divisor = divisor * 2;
            count = count + 1;
        }
        let significand = self.scale + magnitude % self.scale;
        var rounded = significand / divisor;
        let remainder = significand % divisor;
        if remainder > divisor / 2 || (remainder == divisor / 2 && rounded % 2 != 0) {
            rounded = rounded + 1;
        }
        return F { bits= rounded };
    }

    // |reduced| <= ln(2)/2. The degree-20 Taylor remainder is below 2^-95.
    flow exponential<F ~ Float>(self, reduced: F, exponent: i32) -> F ![] {
        var term: F = 1;
        var total: F = 1;
        var divisor: F = 1;
        var index: usize = 0;
        while index < 20 {
            term = term * reduced / divisor;
            total = total + term;
            divisor = divisor + 1;
            index = index + 1;
        }
        return self.scaled(total, exponent);
    }
}

public flow exp<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    if is_nan(x) { return x; }
    let high = encoding.constant<F>(0x42b17218, 0x40862e42fefa39ef);
    let low = encoding.constant<F>(0xc2d00000, 0xc087500000000000);
    if x > high { return F { bits= encoding.exponent_maximum * encoding.scale }; }
    if x < low { return F { bits= 0 }; }
    let inverse_ln2 = encoding.constant<F>(0x3fb8aa3b, 0x3ff71547652b82fe);
    let whole = round(x * inverse_ln2);
    let exponent = FloatEncoding.of(whole).small_integer();
    let ln2_high = encoding.constant<F>(0x3f317000, 0x3fe62e42fee00000);
    let ln2_low = encoding.constant<F>(0x3805fdf4, 0x3dea39ef35793c76);
    let reduced = (x - whole * ln2_high) - whole * ln2_low;
    return encoding.exponential(reduced, exponent);
}

public flow exp2<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    if is_nan(x) { return x; }
    let high = encoding.constant<F>(0x43000000, 0x4090000000000000);
    let low = encoding.constant<F>(0xc3160000, 0xc090cc0000000000);
    if x > high { return F { bits= encoding.exponent_maximum * encoding.scale }; }
    if x < low { return F { bits= 0 }; }
    let whole = round(x);
    let exponent = FloatEncoding.of(whole).small_integer();
    let remainder = x - whole;
    let reduced = remainder * encoding.constant<F>(0x3f317000, 0x3fe62e42fee00000) + remainder * encoding.constant<F>(0x3805fdf4, 0x3dea39ef35793c76);
    return encoding.exponential(reduced, exponent);
}

public flow ln<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    let magnitude = x.bits % encoding.sign;
    let nonfinite = encoding.exponent_maximum * encoding.scale;
    if magnitude > nonfinite { return x; }
    if magnitude == 0 { return F { bits= encoding.sign + nonfinite }; }
    if x.bits >= encoding.sign { return F { bits= nonfinite + encoding.scale / 2 }; }
    if magnitude == nonfinite { return x; }
    let raw_exponent = magnitude / encoding.scale;
    var exponent = i32 { bits= raw_exponent } - i32 { bits= encoding.bias };
    var significand = magnitude % encoding.scale;
    if raw_exponent == 0 {
        exponent = 1 - i32 { bits= encoding.bias };
        while significand < encoding.scale {
            significand = significand * 2;
            exponent = exponent - 1;
        }
    } else {
        significand = significand + encoding.scale;
    }
    var normalized = F { bits= encoding.bias * encoding.scale + significand - encoding.scale };
    if normalized > encoding.constant<F>(0x3fb504f3, 0x3ff6a09e667f3bcd) {
        normalized = normalized / 2;
        exponent = exponent + 1;
    }
    let ratio = (normalized - 1) / (normalized + 1);
    let square = ratio * ratio;
    var power: F = 1;
    var series: F = 1;
    var divisor: F = 3;
    var index: usize = 0;
    while index < 20 {
        power = power * square;
        series = series + power / divisor;
        divisor = divisor + 2;
        index = index + 1;
    }
    let integral = encoding.integer<F>(exponent);
    return integral * encoding.constant<F>(0x3f317000, 0x3fe62e42fee00000) + (2 * ratio * series + integral * encoding.constant<F>(0x3805fdf4, 0x3dea39ef35793c76));
}

public flow log2<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    return ln(x) * encoding.constant<F>(0x3fb8aa3b, 0x3ff71547652b82fe);
}

public flow log10<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    return ln(x) * encoding.constant<F>(0x3ede5bd9, 0x3fdbcb7b1526e50e);
}

public flow pow<F ~ Float>(base: F, exponent: F) -> F ![] {
    let encoding = FloatEncoding.of(base);
    let zero = F { bits= 0 };
    let one: F = 1;
    if exponent == zero || base == one { return one; }
    if is_nan(base) { return base; }
    if is_nan(exponent) { return exponent; }
    let magnitude = F { bits= base.bits % encoding.sign };
    let infinity = encoding.exponent_maximum * encoding.scale;
    if is_infinite(exponent) {
        if magnitude == one { return one; }
        return F { bits= if (magnitude > one) == (exponent > zero) { infinity } else { 0 } };
    }
    let parity = FloatEncoding.of(exponent).integer_parity();
    let odd = match parity { Some(odd) => odd, None => false };
    let sign = if odd && base.bits >= encoding.sign { encoding.sign } else { 0 };
    if magnitude == zero || is_infinite(magnitude) {
        let zero_result = (magnitude == zero) == (exponent > zero);
        return F { bits= sign + if zero_result { 0 } else { infinity } };
    }
    if base < zero && parity.is_none() {
        return F { bits= infinity + encoding.scale / 2 };
    }
    if magnitude == one { return F { bits= one.bits + sign }; }
    var result: F = 1;
    if parity.is_some() && abs(exponent) <= 127 {
        // Small integral powers avoid logarithmic error and reciprocal
        // overflow from first computing a positive power of a large base.
        var remaining = FloatEncoding.of(abs(exponent)).small_integer();
        var factor = if exponent < zero { one / magnitude } else { magnitude };
        while remaining > 0 {
            if remaining % 2 != 0 { result = result * factor; }
            remaining = remaining / 2;
            if remaining != 0 { factor = factor * factor; }
        }
    } else {
        result = exp(exponent * ln(magnitude));
    }
    return F { bits= result.bits + sign };
}
