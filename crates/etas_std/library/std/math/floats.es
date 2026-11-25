module std.math;

// IEEE encoding operations stay in Etas, including rounding and square root.
type FloatEncoding = {
    bits: u128,
    sign: u128,
    scale: u128,
    bias: u128,
    exponent_maximum: u128,
    fraction_width: u128,
};
impl FloatEncoding {
    flow of<F ~ Float>(value: F) -> FloatEncoding ![] {
        let maximum = (F { bits= 340282366920938463463374607431768211455 }).bits;
        let narrow = maximum == 4294967295;
        return FloatEncoding {
            bits= value.bits, sign= maximum / 2 + 1,
            scale= if narrow { 8388608 } else { 4503599627370496 },
            bias= if narrow { 127 } else { 1023 },
            exponent_maximum= if narrow { 255 } else { 2047 },
            fraction_width= if narrow { 23 } else { 52 },
        };
    }
    flow unit(self) -> u128 ![] {
        var unit = self.scale;
        var exponent = (self.bits % self.sign) / self.scale - self.bias;
        while exponent > 0 {
            unit = unit / 2;
            exponent = exponent - 1;
        }
        return unit;
    }
    flow truncated_bits(self) -> u128 ![] {
        let exponent = (self.bits % self.sign) / self.scale;
        if exponent >= self.bias + self.fraction_width { return self.bits; }
        if exponent < self.bias { return if self.bits >= self.sign { self.sign } else { 0 }; }
        return self.bits - self.bits % self.unit();
    }
}

public flow is_nan<F ~ Float>(x: F) -> bool ![] {
    let encoding = FloatEncoding.of(x);
    return encoding.bits % encoding.sign > encoding.exponent_maximum * encoding.scale;
}
public flow is_finite<F ~ Float>(x: F) -> bool ![] {
    let encoding = FloatEncoding.of(x);
    return (encoding.bits % encoding.sign) / encoding.scale < encoding.exponent_maximum;
}
public flow is_infinite<F ~ Float>(x: F) -> bool ![] {
    let encoding = FloatEncoding.of(x);
    return encoding.bits % encoding.sign == encoding.exponent_maximum * encoding.scale;
}
public flow is_normal<F ~ Float>(x: F) -> bool ![] {
    let encoding = FloatEncoding.of(x);
    let exponent = (encoding.bits % encoding.sign) / encoding.scale;
    return exponent > 0 && exponent < encoding.exponent_maximum;
}
public flow is_sign_negative<F ~ Float>(x: F) -> bool ![] {
    let encoding = FloatEncoding.of(x);
    return encoding.bits >= encoding.sign;
}
public flow trunc<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    return F { bits= encoding.truncated_bits() };
}
public flow floor<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    let truncated = encoding.truncated_bits();
    if truncated == encoding.bits { return x; }
    if encoding.bits < encoding.sign { return F { bits= truncated }; }
    if truncated == encoding.sign { return F { bits= encoding.sign + encoding.bias * encoding.scale }; }
    return F { bits= truncated + encoding.unit() };
}
public flow ceil<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    let truncated = encoding.truncated_bits();
    if truncated == encoding.bits { return x; }
    if encoding.bits >= encoding.sign { return F { bits= truncated }; }
    if truncated == 0 { return F { bits= encoding.bias * encoding.scale }; }
    return F { bits= truncated + encoding.unit() };
}
public flow round<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    let truncated = encoding.truncated_bits();
    if truncated == encoding.bits { return x; }
    let magnitude = encoding.bits % encoding.sign;
    let exponent = magnitude / encoding.scale;
    if exponent < encoding.bias {
        if exponent == encoding.bias - 1 && magnitude % encoding.scale != 0 {
            return F { bits= truncated + encoding.bias * encoding.scale };
        }
        return F { bits= truncated };
    }
    let unit = encoding.unit();
    let remainder = encoding.bits - truncated;
    if remainder > unit / 2 || (remainder == unit / 2 && (truncated / unit) % 2 != 0) {
        return F { bits= truncated + unit };
    }
    return F { bits= truncated };
}
public flow sqrt<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    let magnitude = encoding.bits % encoding.sign;
    let nonfinite = encoding.exponent_maximum * encoding.scale;
    if magnitude == 0 || magnitude > nonfinite { return x; }
    if encoding.bits >= encoding.sign {
        return F { bits= nonfinite + encoding.scale / 2 };
    }
    if magnitude == nonfinite { return x; }
    let raw_exponent = magnitude / encoding.scale;
    var exponent = i32 { bits= raw_exponent } - i32 { bits= encoding.bias };
    var mantissa = magnitude % encoding.scale;
    if raw_exponent == 0 {
        exponent = 1 - i32 { bits= encoding.bias };
        while mantissa < encoding.scale {
            mantissa = mantissa * 2;
            exponent = exponent - 1;
        }
    } else {
        mantissa = mantissa + encoding.scale;
    }
    let odd = exponent % 2 != 0;
    var half_exponent = if odd { (exponent - 1) / 2 } else { exponent / 2 };
    let radicand = mantissa * encoding.scale * if odd { 2 } else { 1 };
    var lower = encoding.scale;
    var upper = encoding.scale * 2;
    while lower + 1 < upper {
        let middle = lower + (upper - lower) / 2;
        if middle * middle <= radicand { lower = middle; } else { upper = middle; }
    }
    // An integer radicand cannot lie exactly on the squared half-integer.
    // Compare its exact remainder instead of rounding an intermediate float.
    var significand = if radicand - lower * lower > lower { lower + 1 } else { lower };
    if significand == encoding.scale * 2 {
        significand = encoding.scale;
        half_exponent = half_exponent + 1;
    }
    let result_exponent = u128 { bits= (half_exponent + i32 { bits= encoding.bias }).bits };
    return F { bits= result_exponent * encoding.scale + significand - encoding.scale };
}
public flow hypot<F ~ Float>(x: F, y: F) -> F ![] {
    let a = abs(x);
    let b = abs(y);
    if is_infinite(a) { return a; }
    if is_infinite(b) { return b; }
    if is_nan(a) { return a; }
    if is_nan(b) { return b; }
    let (large, small) = if a >= b { (a, b) } else { (b, a) };
    if small == (F { bits= 0 }) { return large; }
    // Scale before squaring so finite inputs do not overflow an intermediate.
    let ratio = small / large;
    return large * sqrt(1 + ratio * ratio);
}
public flow copysign<F ~ Float>(magnitude: F, sign: F) -> F ![] {
    let encoding = FloatEncoding.of(magnitude);
    let sign_bit = if sign.bits >= encoding.sign { encoding.sign } else { 0 };
    return F { bits= magnitude.bits % encoding.sign + sign_bit };
}
public flow total_cmp<F ~ Float>(a: F, b: F) -> i32 ![] {
    let encoding = FloatEncoding.of(a);
    let left = a.bits;
    let right = b.bits;
    if left == right { return 0; }
    let negative_left = left >= encoding.sign;
    let negative_right = right >= encoding.sign;
    if negative_left != negative_right { return if negative_left { -1 } else { 1 }; }
    if negative_left { return if left > right { -1 } else { 1 }; }
    return if left < right { -1 } else { 1 };
}
public flow pi<F ~ Float>() -> F ![] {
    let encoding = FloatEncoding.of(F { bits= 0 });
    return F { bits= if encoding.fraction_width == 23 { 0x40490fdb } else { 0x400921fb54442d18 } };
}
public flow tau<F ~ Float>() -> F ![] {
    let encoding = FloatEncoding.of(F { bits= 0 });
    return F { bits= if encoding.fraction_width == 23 { 0x40c90fdb } else { 0x401921fb54442d18 } };
}
public flow e<F ~ Float>() -> F ![] {
    let encoding = FloatEncoding.of(F { bits= 0 });
    return F { bits= if encoding.fraction_width == 23 { 0x402df854 } else { 0x4005bf0a8b145769 } };
}
public flow epsilon<F ~ Float>() -> F ![] {
    let encoding = FloatEncoding.of(F { bits= 0 });
    return F { bits= if encoding.fraction_width == 23 { 0x34000000 } else { 0x3cb0000000000000 } };
}
