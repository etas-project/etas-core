module std.math;

public flow sin<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    if is_nan(x) { return x; }
    if is_infinite(x) { return F { bits= encoding.exponent_maximum * encoding.scale + encoding.scale / 2 }; }
    if x.bits % encoding.sign == 0 { return x; }
    let (quadrant, reduced) = encoding.reduced_angle(F { bits= x.bits % encoding.sign });
    let (sine, cosine) = encoding.sine_cosine(reduced);
    let value = if quadrant == 0 { sine } else if quadrant == 1 { cosine } else if quadrant == 2 { -sine } else { -cosine };
    return if x.bits >= encoding.sign { -value } else { value };
}

public flow cos<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    if is_nan(x) { return x; }
    if is_infinite(x) { return F { bits= encoding.exponent_maximum * encoding.scale + encoding.scale / 2 }; }
    let (quadrant, reduced) = encoding.reduced_angle(F { bits= x.bits % encoding.sign });
    let (sine, cosine) = encoding.sine_cosine(reduced);
    return if quadrant == 0 { cosine } else if quadrant == 1 { -sine } else if quadrant == 2 { -cosine } else { sine };
}

public flow tan<F ~ Float>(x: F) -> F ![] {
    let encoding = FloatEncoding.of(x);
    if is_nan(x) { return x; }
    if is_infinite(x) { return F { bits= encoding.exponent_maximum * encoding.scale + encoding.scale / 2 }; }
    if x.bits % encoding.sign == 0 { return x; }
    let (quadrant, reduced) = encoding.reduced_angle(F { bits= x.bits % encoding.sign });
    let (sine, cosine) = encoding.sine_cosine(reduced);
    let value = if quadrant % 2 == 0 { sine / cosine } else { -(cosine / sine) };
    return if x.bits >= encoding.sign { -value } else { value };
}

impl FloatEncoding {
    // After the reciprocal and pi/4 reductions, |x| <= sqrt(2)-1.
    // The first omitted Taylor term is bounded by |x|^51 / 51.
    flow arctangent<F ~ Float>(self, x: F) -> F ![] {
        let square = -(x * x);
        var term = x;
        var total = x;
        var divisor: F = 3;
        var index: usize = 0;
        while index < 24 {
            term = term * square;
            total = total + term / divisor;
            divisor = divisor + 2;
            index = index + 1;
        }
        return total;
    }
}

public flow atan<F ~ Float>(x: F) -> F ![] {
    if is_nan(x) { return x; }
    let encoding = FloatEncoding.of(x);
    let magnitude = F { bits= x.bits % encoding.sign };
    let half_pi = encoding.constant<F>(0x3fc90fdb, 0x3ff921fb54442d18);
    let quarter_pi = encoding.constant<F>(0x3f490fdb, 0x3fe921fb54442d18);
    let reciprocal = magnitude > 1;
    let reduced = if reciprocal { 1 / magnitude } else { magnitude };
    let angle = if reduced > encoding.constant<F>(0x3ed413cd, 0x3fda827999fcef32) {
        quarter_pi + encoding.arctangent((reduced - 1) / (reduced + 1))
    } else {
        encoding.arctangent(reduced)
    };
    return copysign(if reciprocal { half_pi - angle } else { angle }, x);
}

public flow atan2<F ~ Float>(y: F, x: F) -> F ![] {
    if is_nan(y) { return y; }
    if is_nan(x) { return x; }
    let encoding = FloatEncoding.of(x);
    let zero = F { bits= 0 };
    let half_pi = encoding.constant<F>(0x3fc90fdb, 0x3ff921fb54442d18);
    let negative_x = x.bits >= encoding.sign;
    if y == zero { return copysign(if negative_x { pi<F>() } else { zero }, y); }
    if x == zero { return copysign(half_pi, y); }
    if is_infinite(y) {
        if is_infinite(x) {
            let quarter_pi = encoding.constant<F>(0x3f490fdb, 0x3fe921fb54442d18);
            return copysign(if negative_x { pi<F>() - quarter_pi } else { quarter_pi }, y);
        }
        return copysign(half_pi, y);
    }
    if is_infinite(x) { return copysign(if negative_x { pi<F>() } else { zero }, y); }
    let a = abs(y);
    let b = abs(x);
    let angle = if a <= b { atan(a / b) } else { half_pi - atan(b / a) };
    return copysign(if negative_x { pi<F>() - angle } else { angle }, y);
}

public flow asin<F ~ Float>(x: F) -> F ![] {
    if is_nan(x) { return x; }
    if abs(x) > 1 {
        let encoding = FloatEncoding.of(x);
        return F { bits= encoding.exponent_maximum * encoding.scale + encoding.scale / 2 };
    }
    return atan2(x, sqrt((1 - x) * (1 + x)));
}

public flow acos<F ~ Float>(x: F) -> F ![] {
    if is_nan(x) { return x; }
    if abs(x) > 1 {
        let encoding = FloatEncoding.of(x);
        return F { bits= encoding.exponent_maximum * encoding.scale + encoding.scale / 2 };
    }
    return atan2(sqrt((1 - x) * (1 + x)), x);
}
