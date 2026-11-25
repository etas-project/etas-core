module std.math;

public spec Integer ~ Number;
public spec Float ~ Number;

public enum ArithmeticError { Overflow, DivisionByZero, InvalidShift }
public enum MathError { InvalidBounds }

// Integer bit storage is inspected and manipulated using ordinary Etas values.
// u128 holds the complete encoding of every supported target integer width.
type IntegerEncoding = { bits: u128, maximum: u128 };
impl IntegerEncoding {
    flow of<I ~ Integer>(value: I) -> IntegerEncoding ![] {
        return IntegerEncoding { bits= value.bits, maximum= ((I { bits= 340282366920938463463374607431768211455 })).bits };
    }
    flow added(self, other: u128) -> IntegerEncoding ![] {
        let distance = self.maximum - other;
        let bits = if self.bits > distance { self.bits - distance - 1 } else { self.bits + other };
        return IntegerEncoding { bits= bits, maximum= self.maximum };
    }
    flow subtracted(self, other: u128) -> IntegerEncoding ![] {
        let bits = if self.bits >= other { self.bits - other } else { self.maximum - (other - self.bits) + 1 };
        return IntegerEncoding { bits= bits, maximum= self.maximum };
    }
    flow multiplied(self, other: u128) -> IntegerEncoding ![] {
        var output = IntegerEncoding { bits= 0, maximum= self.maximum };
        var factor = self;
        var remaining = other;
        while remaining > 0 {
            if remaining % 2 != 0 { output = output.added(factor.bits); }
            remaining = remaining / 2;
            if remaining > 0 { factor = factor.added(factor.bits); }
        }
        return output;
    }
    flow width(self) -> u32 ![] {
        var remaining = self.maximum;
        var width: u32 = 0;
        while remaining > 0 {
            remaining = remaining / 2;
            width = width + 1;
        }
        return width;
    }
}

public flow min_value<I ~ Integer>() -> I ![] {
    let all = (I { bits= 340282366920938463463374607431768211455 });
    let zero = (I { bits= 0 });
    return if all < zero { I { bits= all.bits / 2 + 1 } } else { zero };
}
public flow max_value<I ~ Integer>() -> I ![] {
    let all = (I { bits= 340282366920938463463374607431768211455 });
    return if all < (I { bits= 0 }) { I { bits= all.bits / 2 } } else { all };
}
public flow bit_width<I ~ Integer>() -> u32 ![] {
    return IntegerEncoding.of((I { bits= 0 })).width();
}
public flow checked_add<I ~ Integer>(a: I, b: I) -> Result<I, ArithmeticError> ![] {
    let zero = (I { bits= 0 });
    if (b > zero && a > max_value<I>() - b) || (b < zero && a < min_value<I>() - b) {
        return Err(ArithmeticError.Overflow);
    }
    return Ok(a + b);
}
public flow checked_sub<I ~ Integer>(a: I, b: I) -> Result<I, ArithmeticError> ![] {
    let zero = (I { bits= 0 });
    if (b > zero && a < min_value<I>() + b) || (b < zero && a > max_value<I>() + b) {
        return Err(ArithmeticError.Overflow);
    }
    return Ok(a - b);
}
public flow checked_mul<I ~ Integer>(a: I, b: I) -> Result<I, ArithmeticError> ![] {
    let zero = (I { bits= 0 });
    if a == zero || b == zero { return Ok(zero); }
    if a > zero {
        if b > zero {
            if a > max_value<I>() / b { return Err(ArithmeticError.Overflow); }
        } else {
            if b < min_value<I>() / a { return Err(ArithmeticError.Overflow); }
        }
    } else {
        if b > zero {
            if a < min_value<I>() / b { return Err(ArithmeticError.Overflow); }
        } else {
            if a < max_value<I>() / b { return Err(ArithmeticError.Overflow); }
        }
    }
    return Ok(a * b);
}
public flow checked_div<I ~ Integer>(a: I, b: I) -> Result<I, ArithmeticError> ![] {
    let zero = (I { bits= 0 });
    if b == zero { return Err(ArithmeticError.DivisionByZero); }
    if b < zero && a == min_value<I>() && b == (I { bits= 340282366920938463463374607431768211455 }) {
        return Err(ArithmeticError.Overflow);
    }
    return Ok(a / b);
}
public flow checked_rem<I ~ Integer>(a: I, b: I) -> Result<I, ArithmeticError> ![] {
    let zero = (I { bits= 0 });
    if b == zero { return Err(ArithmeticError.DivisionByZero); }
    if b < zero && a == min_value<I>() && b == (I { bits= 340282366920938463463374607431768211455 }) {
        return Ok(zero);
    }
    return Ok(a % b);
}
public flow checked_abs<I ~ Integer>(value: I) -> Result<I, ArithmeticError> ![] {
    let zero = (I { bits= 0 });
    if value >= zero { return Ok(value); }
    if value == min_value<I>() { return Err(ArithmeticError.Overflow); }
    return Ok(zero - value);
}
public flow checked_pow<I ~ Integer>(base: I, exponent: u32) -> Result<I, ArithmeticError> ![] {
    var output = I { bits= 1 };
    var factor = base;
    var remaining = exponent;
    while remaining > 0 {
        if remaining % 2 != 0 {
            output = match checked_mul(output, factor) { Ok(value) => value, Err(error) => { return Err(error); } };
        }
        remaining = remaining / 2;
        if remaining > 0 {
            factor = match checked_mul(factor, factor) { Ok(value) => value, Err(error) => { return Err(error); } };
        }
    }
    return Ok(output);
}
public flow div_euclid<I ~ Integer>(a: I, b: I) -> Result<I, ArithmeticError> ![] {
    let quotient = match checked_div(a, b) { Ok(value) => value, Err(error) => { return Err(error); } };
    let remainder = match checked_rem(a, b) { Ok(value) => value, Err(error) => { return Err(error); } };
    let zero = (I { bits= 0 });
    if remainder < zero {
        return if b > zero { checked_sub(quotient, I { bits= 1 }) } else { checked_add(quotient, I { bits= 1 }) };
    }
    return Ok(quotient);
}
public flow rem_euclid<I ~ Integer>(a: I, b: I) -> Result<I, ArithmeticError> ![] {
    let remainder = match checked_rem(a, b) { Ok(value) => value, Err(error) => { return Err(error); } };
    let zero = (I { bits= 0 });
    if remainder < zero {
        return if b > zero { checked_add(remainder, b) } else { checked_sub(remainder, b) };
    }
    return Ok(remainder);
}
public flow wrapping_add<I ~ Integer>(a: I, b: I) -> I ![] {
    return I { bits= IntegerEncoding.of(a).added(b.bits).bits };
}
public flow wrapping_sub<I ~ Integer>(a: I, b: I) -> I ![] {
    return I { bits= IntegerEncoding.of(a).subtracted(b.bits).bits };
}
public flow wrapping_mul<I ~ Integer>(a: I, b: I) -> I ![] {
    return I { bits= IntegerEncoding.of(a).multiplied(b.bits).bits };
}
public flow saturating_add<I ~ Integer>(a: I, b: I) -> I ![] {
    return match checked_add(a, b) {
        Ok(value) => value,
        Err(_) => if b >= (I { bits= 0 }) { max_value<I>() } else { min_value<I>() },
    };
}
public flow saturating_sub<I ~ Integer>(a: I, b: I) -> I ![] {
    return match checked_sub(a, b) {
        Ok(value) => value,
        Err(_) => if b >= (I { bits= 0 }) { min_value<I>() } else { max_value<I>() },
    };
}
public flow saturating_mul<I ~ Integer>(a: I, b: I) -> I ![] {
    return match checked_mul(a, b) {
        Ok(value) => value,
        Err(_) => if (a < (I { bits= 0 })) != (b < (I { bits= 0 })) { min_value<I>() } else { max_value<I>() },
    };
}
public flow checked_shl<I ~ Integer>(value: I, count: u32) -> Result<I, ArithmeticError> ![] {
    var encoding = IntegerEncoding.of(value);
    if count >= encoding.width() { return Err(ArithmeticError.InvalidShift); }
    var remaining = count;
    while remaining > 0 {
        encoding = encoding.added(encoding.bits);
        remaining = remaining - 1;
    }
    return Ok(I { bits= encoding.bits });
}
public flow checked_shr<I ~ Integer>(value: I, count: u32) -> Result<I, ArithmeticError> ![] {
    let encoding = IntegerEncoding.of(value);
    if count >= encoding.width() { return Err(ArithmeticError.InvalidShift); }
    let extension = if value < (I { bits= 0 }) { encoding.maximum / 2 + 1 } else { 0 };
    var bits = encoding.bits;
    var remaining = count;
    while remaining > 0 {
        bits = bits / 2 + extension;
        remaining = remaining - 1;
    }
    return Ok(I { bits= bits });
}
public flow rotate_left<I ~ Integer>(value: I, count: u32) -> I ![] {
    var encoding = IntegerEncoding.of(value);
    let high = encoding.maximum / 2 + 1;
    var remaining = count % encoding.width();
    while remaining > 0 {
        let carry: u128 = if encoding.bits >= high { 1 } else { 0 };
        encoding = encoding.added(encoding.bits).added(carry);
        remaining = remaining - 1;
    }
    return I { bits= encoding.bits };
}
public flow rotate_right<I ~ Integer>(value: I, count: u32) -> I ![] {
    let encoding = IntegerEncoding.of(value);
    let high = encoding.maximum / 2 + 1;
    var bits = encoding.bits;
    var remaining = count % encoding.width();
    while remaining > 0 {
        let carry = if bits % 2 != 0 { high } else { 0 };
        bits = bits / 2 + carry;
        remaining = remaining - 1;
    }
    return I { bits= bits };
}
public flow count_ones<I ~ Integer>(value: I) -> u32 ![] {
    var bits = value.bits;
    var count: u32 = 0;
    while bits > 0 {
        if bits % 2 != 0 { count = count + 1; }
        bits = bits / 2;
    }
    return count;
}
public flow leading_zeros<I ~ Integer>(value: I) -> u32 ![] {
    let encoding = IntegerEncoding.of(value);
    var high = encoding.maximum / 2 + 1;
    var count: u32 = 0;
    while high > 0 && encoding.bits < high {
        count = count + 1;
        high = high / 2;
    }
    return count;
}
public flow trailing_zeros<I ~ Integer>(value: I) -> u32 ![] {
    let encoding = IntegerEncoding.of(value);
    if encoding.bits == 0 { return encoding.width(); }
    var bits = encoding.bits;
    var count: u32 = 0;
    while bits % 2 == 0 {
        count = count + 1;
        bits = bits / 2;
    }
    return count;
}
public flow clamp<T ~ Number>(value: T, lower: T, upper: T) -> Result<T, MathError> ![] {
    if !(value <= value) || !(lower <= lower) || !(upper <= upper) || lower > upper {
        return Err(MathError.InvalidBounds);
    }
    if value < lower { return Ok(lower); }
    if value > upper { return Ok(upper); }
    return Ok(value);
}
