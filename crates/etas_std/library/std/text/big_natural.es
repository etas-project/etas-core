module std.text;

// Little-endian base-2^32 limbs. All arithmetic and normalization are Etas.
let FLOAT_POWERS: Array<u128> = [
    1,
    2,
    4,
    8,
    16,
    32,
    64,
    128,
    256,
    512,
    1024,
    2048,
    4096,
    8192,
    16384,
    32768,
    65536,
    131072,
    262144,
    524288,
    1048576,
    2097152,
    4194304,
    8388608,
    16777216,
    33554432,
    67108864,
    134217728,
    268435456,
    536870912,
    1073741824,
    2147483648,
    4294967296,
    8589934592,
    17179869184,
    34359738368,
    68719476736,
    137438953472,
    274877906944,
    549755813888,
    1099511627776,
    2199023255552,
    4398046511104,
    8796093022208,
    17592186044416,
    35184372088832,
    70368744177664,
    140737488355328,
    281474976710656,
    562949953421312,
    1125899906842624,
    2251799813685248,
    4503599627370496,
    9007199254740992,
    18014398509481984,
    36028797018963968,
    72057594037927936,
    144115188075855872,
    288230376151711744,
    576460752303423488,
    1152921504606846976,
    2305843009213693952,
    4611686018427387904,
    9223372036854775808,
    18446744073709551616,
];

type BigNat = { private words: Array<u128> };

impl BigNat {
    flow new(value: u128) -> BigNat ![] {
        var remaining = value;
        var words: Array<u128> = [];
        while remaining != 0 {
            words = words.push(remaining % 4294967296);
            remaining = remaining / 4294967296;
        }
        return BigNat { words= words };
    }

    flow canonical(words: Array<u128>) -> BigNat ![] {
        var last: usize = 0;
        var index: usize = 0;
        for word in words {
            index = index + 1;
            if word != 0 { last = index; }
        }
        if last == words.length { return BigNat { words= words }; }
        var output: Array<u128> = [];
        index = 0;
        while index < last {
            output = output.push(words.get(index).unwrap());
            index = index + 1;
        }
        return BigNat { words= output };
    }

    flow is_zero(self) -> bool ![] {
        return self.words.length == 0;
    }

    flow compare(self, other: BigNat) -> i32 ![] {
        if self.words.length < other.words.length { return -1; }
        if self.words.length > other.words.length { return 1; }
        var index = self.words.length;
        while index != 0 {
            index = index - 1;
            let left = self.words.get(index).unwrap();
            let right = other.words.get(index).unwrap();
            if left < right { return -1; }
            if left > right { return 1; }
        }
        return 0;
    }

    flow bit_length(self) -> usize ![] {
        if self.is_zero() { return 0; }
        var high = self.words.get(self.words.length - 1).unwrap();
        var bits: usize = 0;
        while high != 0 {
            high = high / 2;
            bits = bits + 1;
        }
        return (self.words.length - 1) * 32 + bits;
    }

    flow multiply_small(self, factor: u128) -> BigNat ![] {
        if factor == 0 { return BigNat { words= [] }; }
        var carry: u128 = 0;
        var output: Array<u128> = [];
        for word in self.words {
            let product = word * factor + carry;
            output = output.push(product % 4294967296);
            carry = product / 4294967296;
        }
        while carry != 0 {
            output = output.push(carry % 4294967296);
            carry = carry / 4294967296;
        }
        return BigNat { words= output };
    }

    flow add_small(self, value: u128) -> BigNat ![] {
        var carry = value;
        var output: Array<u128> = [];
        for word in self.words {
            let sum = word + carry;
            output = output.push(sum % 4294967296);
            carry = sum / 4294967296;
        }
        while carry != 0 {
            output = output.push(carry % 4294967296);
            carry = carry / 4294967296;
        }
        return BigNat { words= output };
    }

    flow power_scale(self, factor: u128, count: usize) -> BigNat ![] {
        var output = self;
        var index: usize = 0;
        while index < count {
            output = output.multiply_small(factor);
            index = index + 1;
        }
        return output;
    }

    flow shift_left(self, distance: usize) -> BigNat ![] {
        if self.is_zero() { return self; }
        var output: Array<u128> = [];
        var index: usize = 0;
        while index < distance / 32 {
            output = output.push(0);
            index = index + 1;
        }
        let factor = FLOAT_POWERS.get(distance % 32).unwrap();
        var carry: u128 = 0;
        for word in self.words {
            let shifted = word * factor + carry;
            output = output.push(shifted % 4294967296);
            carry = shifted / 4294967296;
        }
        if carry != 0 { output = output.push(carry); }
        return BigNat { words= output };
    }

    flow subtract(self, other: BigNat) -> BigNat ![] {
        var output: Array<u128> = [];
        var borrow: u128 = 0;
        var index: usize = 0;
        while index < self.words.length {
            let left = self.words.get(index).unwrap();
            let right: u128 = match other.words.get(index) { Some(word) => word, None => 0 };
            let required = right + borrow;
            if left < required {
                output = output.push(4294967296 + left - required);
                borrow = 1;
            } else {
                output = output.push(left - required);
                borrow = 0;
            }
            index = index + 1;
        }
        return BigNat.canonical(output);
    }

    // Used only after scaling a float ratio to a <= 54-bit significand.
    flow divide_significand(self, divisor: BigNat) -> (u128, BigNat) ![] {
        if self.compare(divisor) < 0 { return (0, self); }
        var distance = self.bit_length() - divisor.bit_length();
        var bit = FLOAT_POWERS.get(distance).unwrap();
        var remainder = self;
        var quotient: u128 = 0;
        while true {
            let shifted = divisor.shift_left(distance);
            if remainder.compare(shifted) >= 0 {
                remainder = remainder.subtract(shifted);
                quotient = quotient + bit;
            }
            if distance == 0 { return (quotient, remainder); }
            distance = distance - 1;
            bit = bit / 2;
        }
    }

    flow divide_small(self, divisor: u128) -> (BigNat, u128) ![] {
        var index = self.words.length;
        var carry: u128 = 0;
        var words: List<u128> = [];
        while index != 0 {
            index = index - 1;
            let dividend = carry * 4294967296 + self.words.get(index).unwrap();
            words = (dividend / divisor) :: words;
            carry = dividend % divisor;
        }
        var output: Array<u128> = [];
        for word in words {
            output = output.push(word);
        }
        return (BigNat.canonical(output), carry);
    }

    flow decimal(self) -> string ![] {
        if self.is_zero() { return "0"; }
        var remaining = self;
        var output = "";
        while !remaining.is_zero() {
            let (next, digit) = remaining.divide_small(10);
            output = DECIMAL_DIGITS.get(digit).unwrap() + output;
            remaining = next;
        }
        return output;
    }
}
