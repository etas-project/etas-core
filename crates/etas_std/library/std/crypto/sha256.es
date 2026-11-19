module std.crypto;

// Fixed-width boolean arithmetic is source code. Primitive arithmetic, raw
// numeric representation and byte storage are the only execution mechanisms.
let BIT_POWERS: Array<u128> = [
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
];

let BYTE_POWERS: Array<u128> = [
    1,
    256,
    65536,
    16777216,
    4294967296,
    1099511627776,
    281474976710656,
    72057594037927936,
];

type Word32 = { private raw: u128 };

impl Word32 {
    flow add(self, other: Word32) -> Word32 ![] {
        return Word32 { raw= (self.raw + other.raw) % 4294967296 };
    }

    flow xor(self, other: Word32) -> Word32 ![] {
        var left = self.raw;
        var right = other.raw;
        var place: u128 = 1;
        var output: u128 = 0;
        var bit: usize = 0;
        while bit < 32 limit Iterations(32) {
            output = output + ((left % 2 + right % 2) % 2) * place;
            left = left / 2;
            right = right / 2;
            place = place * 2;
            bit = bit + 1;
        }
        return Word32 { raw= output };
    }

    flow and(self, other: Word32) -> Word32 ![] {
        var left = self.raw;
        var right = other.raw;
        var place: u128 = 1;
        var output: u128 = 0;
        var bit: usize = 0;
        while bit < 32 limit Iterations(32) {
            output = output + (left % 2) * (right % 2) * place;
            left = left / 2;
            right = right / 2;
            place = place * 2;
            bit = bit + 1;
        }
        return Word32 { raw= output };
    }

    flow rotate_right(self, distance: usize) -> Word32 ![] {
        let low = BIT_POWERS.get(distance).unwrap();
        let high = BIT_POWERS.get(32 - distance).unwrap();
        return Word32 { raw= self.raw / low + (self.raw % low) * high };
    }

    flow shift_right(self, distance: usize) -> Word32 ![] {
        return Word32 { raw= self.raw / BIT_POWERS.get(distance).unwrap() };
    }
}

let ROUND_CONSTANTS: Array<Word32> = [
    Word32 { raw= 1116352408 },
    Word32 { raw= 1899447441 },
    Word32 { raw= 3049323471 },
    Word32 { raw= 3921009573 },
    Word32 { raw= 961987163 },
    Word32 { raw= 1508970993 },
    Word32 { raw= 2453635748 },
    Word32 { raw= 2870763221 },
    Word32 { raw= 3624381080 },
    Word32 { raw= 310598401 },
    Word32 { raw= 607225278 },
    Word32 { raw= 1426881987 },
    Word32 { raw= 1925078388 },
    Word32 { raw= 2162078206 },
    Word32 { raw= 2614888103 },
    Word32 { raw= 3248222580 },
    Word32 { raw= 3835390401 },
    Word32 { raw= 4022224774 },
    Word32 { raw= 264347078 },
    Word32 { raw= 604807628 },
    Word32 { raw= 770255983 },
    Word32 { raw= 1249150122 },
    Word32 { raw= 1555081692 },
    Word32 { raw= 1996064986 },
    Word32 { raw= 2554220882 },
    Word32 { raw= 2821834349 },
    Word32 { raw= 2952996808 },
    Word32 { raw= 3210313671 },
    Word32 { raw= 3336571891 },
    Word32 { raw= 3584528711 },
    Word32 { raw= 113926993 },
    Word32 { raw= 338241895 },
    Word32 { raw= 666307205 },
    Word32 { raw= 773529912 },
    Word32 { raw= 1294757372 },
    Word32 { raw= 1396182291 },
    Word32 { raw= 1695183700 },
    Word32 { raw= 1986661051 },
    Word32 { raw= 2177026350 },
    Word32 { raw= 2456956037 },
    Word32 { raw= 2730485921 },
    Word32 { raw= 2820302411 },
    Word32 { raw= 3259730800 },
    Word32 { raw= 3345764771 },
    Word32 { raw= 3516065817 },
    Word32 { raw= 3600352804 },
    Word32 { raw= 4094571909 },
    Word32 { raw= 275423344 },
    Word32 { raw= 430227734 },
    Word32 { raw= 506948616 },
    Word32 { raw= 659060556 },
    Word32 { raw= 883997877 },
    Word32 { raw= 958139571 },
    Word32 { raw= 1322822218 },
    Word32 { raw= 1537002063 },
    Word32 { raw= 1747873779 },
    Word32 { raw= 1955562222 },
    Word32 { raw= 2024104815 },
    Word32 { raw= 2227730452 },
    Word32 { raw= 2361852424 },
    Word32 { raw= 2428436474 },
    Word32 { raw= 2756734187 },
    Word32 { raw= 3204031479 },
    Word32 { raw= 3329325298 },
];

type Sha256State = { private words: Array<Word32> };

impl Sha256State {
    flow new() -> Sha256State ![] {
        return Sha256State { words= [
            Word32 { raw= 1779033703 },
            Word32 { raw= 3144134277 },
            Word32 { raw= 1013904242 },
            Word32 { raw= 2773480762 },
            Word32 { raw= 1359893119 },
            Word32 { raw= 2600822924 },
            Word32 { raw= 528734635 },
            Word32 { raw= 1541459225 },
] };
    }

    flow compress(self, input: bytes, start: usize) -> Sha256State ![] {
        var schedule: Array<Word32> = [];
        var index: usize = 0;
        while index < 16 limit Iterations(16) {
            let offset = start + index * 4;
            let value = input.get(offset).unwrap().bits * 16777216
                + input.get(offset + 1).unwrap().bits * 65536
                + input.get(offset + 2).unwrap().bits * 256
                + input.get(offset + 3).unwrap().bits;
            schedule = schedule.push(Word32 { raw= value });
            index = index + 1;
        }
        while index < 64 limit Iterations(48) {
            let a = schedule.get(index - 15).unwrap();
            let b = schedule.get(index - 2).unwrap();
            let sigma0 = a.rotate_right(7).xor(a.rotate_right(18)).xor(a.shift_right(3));
            let sigma1 = b.rotate_right(17).xor(b.rotate_right(19)).xor(b.shift_right(10));
            let next = schedule.get(index - 16).unwrap().add(sigma0)
                .add(schedule.get(index - 7).unwrap()).add(sigma1);
            schedule = schedule.push(next);
            index = index + 1;
        }

        var a = self.words.get(0).unwrap();
        var b = self.words.get(1).unwrap();
        var c = self.words.get(2).unwrap();
        var d = self.words.get(3).unwrap();
        var e = self.words.get(4).unwrap();
        var f = self.words.get(5).unwrap();
        var g = self.words.get(6).unwrap();
        var h = self.words.get(7).unwrap();
        index = 0;
        while index < 64 limit Iterations(64) {
            let sigma1 = e.rotate_right(6).xor(e.rotate_right(11)).xor(e.rotate_right(25));
            let choose = g.xor(e.and(f.xor(g)));
            let temp1 = h.add(sigma1).add(choose)
                .add(ROUND_CONSTANTS.get(index).unwrap()).add(schedule.get(index).unwrap());
            let sigma0 = a.rotate_right(2).xor(a.rotate_right(13)).xor(a.rotate_right(22));
            let majority = a.and(b).xor(a.xor(b).and(c));
            let temp2 = sigma0.add(majority);
            h = g;
            g = f;
            f = e;
            e = d.add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.add(temp2);
            index = index + 1;
        }
        return Sha256State { words= [
            self.words.get(0).unwrap().add(a),
            self.words.get(1).unwrap().add(b),
            self.words.get(2).unwrap().add(c),
            self.words.get(3).unwrap().add(d),
            self.words.get(4).unwrap().add(e),
            self.words.get(5).unwrap().add(f),
            self.words.get(6).unwrap().add(g),
            self.words.get(7).unwrap().add(h),
        ] };
    }
}
