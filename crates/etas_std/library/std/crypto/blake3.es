module std.crypto;

// Source tree hashing and derive-key hashing for existing storage fingerprints.
// Compression, schedule, tree policy and byte encoding are all Etas code.
public type Blake3 = {};

let BLAKE3_IV: Array<u128> = [
    1779033703, 3144134277, 1013904242, 2773480762,
    1359893119, 2600822924, 528734635, 1541459225,
];

alias Blake3Words = (
    u128, u128, u128, u128, u128, u128, u128, u128,
    u128, u128, u128, u128, u128, u128, u128, u128
);

type Blake3Output = {
    private chaining: Array<u128>,
    private block: Array<u128>,
    private counter: u128,
    private length: u128,
    private flags: u128,
};

impl Blake3Output {
    flow mix(a: u128, b: u128, c: u128, d: u128, x: u128, y: u128) -> (u128, u128, u128, u128) ![] {
        let a1 = (a + b + x) % 4294967296;
        let t16 = d ^ a1;
        let d1 = t16 / 65536 + (t16 % 65536) * 65536;
        let c1 = (c + d1) % 4294967296;
        let t12 = b ^ c1;
        let b1 = t12 / 4096 + (t12 % 4096) * 1048576;
        let a2 = (a1 + b1 + y) % 4294967296;
        let t8 = d1 ^ a2;
        let d2 = t8 / 256 + (t8 % 256) * 16777216;
        let c2 = (c1 + d2) % 4294967296;
        let t7 = b1 ^ c2;
        let b2 = t7 / 128 + (t7 % 128) * 33554432;
        return (a2, b2, c2, d2);
    }

    flow round(state: Blake3Words, message: Blake3Words) -> Blake3Words ![] {
        let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15) = state;
        let (m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15) = message;
        let (c0, c4, c8, c12) = Blake3Output.mix(v0, v4, v8, v12, m0, m1);
        let (c1, c5, c9, c13) = Blake3Output.mix(v1, v5, v9, v13, m2, m3);
        let (c2, c6, c10, c14) = Blake3Output.mix(v2, v6, v10, v14, m4, m5);
        let (c3, c7, c11, c15) = Blake3Output.mix(v3, v7, v11, v15, m6, m7);
        let (d0, d5, d10, d15) = Blake3Output.mix(c0, c5, c10, c15, m8, m9);
        let (d1, d6, d11, d12) = Blake3Output.mix(c1, c6, c11, c12, m10, m11);
        let (d2, d7, d8, d13) = Blake3Output.mix(c2, c7, c8, c13, m12, m13);
        let (d3, d4, d9, d14) = Blake3Output.mix(c3, c4, c9, c14, m14, m15);
        return (d0, d1, d2, d3, d4, d5, d6, d7, d8, d9, d10, d11, d12, d13, d14, d15);
    }

    flow permute(message: Blake3Words) -> Blake3Words ![] {
        let (m0, m1, m2, m3, m4, m5, m6, m7, m8, m9, m10, m11, m12, m13, m14, m15) = message;
        return (m2, m6, m3, m10, m7, m0, m4, m13, m1, m11, m12, m5, m9, m14, m15, m8);
    }

    flow compress(self, root: bool) -> Array<u128> ![] {
        let flags = self.flags + if root { 8 } else { 0 };
        var state: Blake3Words = (
            self.chaining[0]?.unwrap(), self.chaining[1]?.unwrap(), self.chaining[2]?.unwrap(), self.chaining[3]?.unwrap(),
            self.chaining[4]?.unwrap(), self.chaining[5]?.unwrap(), self.chaining[6]?.unwrap(), self.chaining[7]?.unwrap(),
            1779033703, 3144134277, 1013904242, 2773480762,
            self.counter % 4294967296, self.counter / 4294967296, self.length, flags
        );
        var message: Blake3Words = (
            self.block[0]?.unwrap(), self.block[1]?.unwrap(), self.block[2]?.unwrap(), self.block[3]?.unwrap(),
            self.block[4]?.unwrap(), self.block[5]?.unwrap(), self.block[6]?.unwrap(), self.block[7]?.unwrap(),
            self.block[8]?.unwrap(), self.block[9]?.unwrap(), self.block[10]?.unwrap(), self.block[11]?.unwrap(),
            self.block[12]?.unwrap(), self.block[13]?.unwrap(), self.block[14]?.unwrap(), self.block[15]?.unwrap()
        );
        var remaining: usize = 7;
        while remaining > 1 {
            state = Blake3Output.round(state, message);
            message = Blake3Output.permute(message);
            remaining = remaining - 1;
        }
        let (v0, v1, v2, v3, v4, v5, v6, v7, v8, v9, v10, v11, v12, v13, v14, v15) = Blake3Output.round(state, message);
        return [v0 ^ v8, v1 ^ v9, v2 ^ v10, v3 ^ v11, v4 ^ v12, v5 ^ v13, v6 ^ v14, v7 ^ v15];
    }
}

impl Blake3 {
    flow block(input: bytes, offset: usize, length: usize) -> Array<u128> ![] {
        var words: Array<u128> = [];
        var index = offset;
        let end = offset + length;
        while end - index >= 4 {
            let word = input[index]?.unwrap().bits
                + input[index + 1]?.unwrap().bits * 256
                + input[index + 2]?.unwrap().bits * 65536
                + input[index + 3]?.unwrap().bits * 16777216;
            words = words.push(word);
            index = index + 4;
        }
        if index < end {
            var word = input[index]?.unwrap().bits;
            if end - index > 1 { word = word + input[index + 1]?.unwrap().bits * 256; }
            if end - index > 2 { word = word + input[index + 2]?.unwrap().bits * 65536; }
            words = words.push(word);
        }
        while words.length < 16 { words = words.push(0); }
        return words;
    }

    flow chunk(input: bytes, offset: usize, length: usize, counter: u128, key: Array<u128>, flags: u128) -> Blake3Output ![] {
        var chaining = key;
        var consumed: usize = 0;
        var start: u128 = 1;
        while length - consumed > 64 {
            let output = Blake3Output {
                chaining= chaining, block= Blake3.block(input, offset + consumed, 64),
                counter= counter, length= 64, flags= flags + start,
            };
            chaining = output.compress(false);
            consumed = consumed + 64;
            start = 0;
        }
        return Blake3Output {
            chaining= chaining, block= Blake3.block(input, offset + consumed, length - consumed),
            counter= counter, length= (length - consumed).bits, flags= flags + start + 2,
        };
    }

    flow parent(left: Array<u128>, right: Array<u128>, key: Array<u128>, flags: u128) -> Blake3Output ![] {
        return Blake3Output { chaining= key, block= left + right, counter= 0, length= 64, flags= flags + 4 };
    }

    flow hash_internal(input: bytes, key: Array<u128>, flags: u128) -> bytes ![] {
        var stack: List<Array<u128>> = [];
        var offset: usize = 0;
        var counter: u128 = 0;
        while input.length - offset > 1024 {
            var chaining = Blake3.chunk(input, offset, 1024, counter, key, flags).compress(false);
            var chunks = counter + 1;
            while chunks % 2 == 0 {
                match stack {
                    left :: tail => {
                        chaining = Blake3.parent(left, chaining, key, flags).compress(false);
                        stack = tail;
                    },
                    [] => std.core.abort("BLAKE3 tree stack invariant"),
                };
                chunks = chunks / 2;
            }
            stack = chaining :: stack;
            offset = offset + 1024;
            counter = counter + 1;
        }
        var output = Blake3.chunk(input, offset, input.length - offset, counter, key, flags);
        for left in stack {
            output = Blake3.parent(left, output.compress(false), key, flags);
        }
        var bytes_out: Array<u8> = [];
        for word in output.compress(true) {
            bytes_out = bytes_out.extend([
                u8 { bits= word % 256 }, u8 { bits= (word / 256) % 256 },
                u8 { bits= (word / 65536) % 256 }, u8 { bits= word / 16777216 },
            ]);
        }
        return bytes { data= bytes_out };
    }

    public flow hash(input: bytes) -> Digest ![] {
        return Blake3.hash_internal(input, BLAKE3_IV, 0);
    }

    public flow derive_key(context: string, input: bytes) -> Digest ![] {
        let context_key = Blake3.hash_internal(context.utf8, BLAKE3_IV, 32);
        let words = Blake3.block(context_key, 0, 32);
        var key: Array<u128> = [];
        var index: usize = 0;
        while index < 8 {
            key = key.push(words.get(index).unwrap());
            index = index + 1;
        }
        return Blake3.hash_internal(input, key, 64);
    }
}
