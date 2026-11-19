module std.math;

public spec Number;

public flow abs<T ~ Number>(value: T) -> T ![] {
    let zero = T { bits= 0 };
    // Unordered comparison identifies NaN without changing its encoding.
    if !(value <= value) {
        let bits = value.bits;
        // Every f32 NaN fits in 32 bits; every f64 NaN exceeds that range.
        let sign: u128 = if bits <= 4294967295 { 2147483648 } else { 9223372036854775808 };
        return T { bits= bits % sign };
    }
    // Checked subtraction also clears either zero sign and faults for a signed minimum.
    if value <= zero { return zero - value; }
    return value;
}

public flow min<T ~ Number>(left: T, right: T) -> T ![] {
    if !(left <= left) {
        return if !(right <= right) { left } else { right };
    }
    if !(right <= right) { return left; }
    if left < right { return left; }
    if right < left { return right; }
    // Among equal numeric values, only opposite floating zeros differ in bits.
    return if left.bits >= right.bits { left } else { right };
}

public flow max<T ~ Number>(left: T, right: T) -> T ![] {
    if !(left <= left) {
        return if !(right <= right) { left } else { right };
    }
    if !(right <= right) { return left; }
    if left > right { return left; }
    if right > left { return right; }
    return if left.bits <= right.bits { left } else { right };
}
