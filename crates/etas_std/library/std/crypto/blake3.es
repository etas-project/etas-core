module std.crypto;

// Source tree hashing and derive-key hashing for existing storage fingerprints.
// Compression, schedule, tree policy and byte encoding are all Etas code.
public type Blake3 = {};

let BLAKE3_IV: Array<Word32> = [
    Word32 { raw= 1779033703 }, Word32 { raw= 3144134277 },
    Word32 { raw= 1013904242 }, Word32 { raw= 2773480762 },
    Word32 { raw= 1359893119 }, Word32 { raw= 2600822924 },
    Word32 { raw= 528734635 }, Word32 { raw= 1541459225 },
];

let BLAKE3_SCHEDULE: Array<Array<usize>> = [

    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8],
    [3, 4, 10, 12, 13, 2, 7, 14, 6, 5, 9, 0, 11, 15, 8, 1],
    [10, 7, 12, 9, 14, 3, 13, 15, 4, 0, 11, 2, 5, 8, 1, 6],
    [12, 13, 9, 11, 15, 10, 14, 8, 7, 2, 5, 3, 0, 1, 6, 4],
    [9, 14, 11, 5, 8, 12, 15, 1, 13, 3, 0, 10, 2, 6, 4, 7],
    [11, 15, 5, 0, 1, 9, 8, 6, 14, 10, 2, 12, 3, 4, 7, 13],
];

type Blake3State = { private words: Array<Word32> };

type Blake3Output = {
    private chaining: Array<Word32>,
    private block: Array<Word32>,
    private counter: u128,
    private length: u128,
    private flags: u128,
};

impl Blake3State {
    flow store(var self, index: usize, word: Word32) -> unit ![] {
        self.words = match self.words.set(index, word)? {
            Ok(words) => words,
            Err(_) => std.core.abort("BLAKE3 state index invariant"),
        };
    }

    flow mix(var self, a: usize, b: usize, c: usize, d: usize, x: Word32, y: Word32) -> unit ![] {
        self.store(a, self.words.get(a).unwrap().add(self.words.get(b).unwrap()).add(x));
        self.store(d, self.words.get(d).unwrap().xor(self.words.get(a).unwrap()).rotate_right(16));
        self.store(c, self.words.get(c).unwrap().add(self.words.get(d).unwrap()));
        self.store(b, self.words.get(b).unwrap().xor(self.words.get(c).unwrap()).rotate_right(12));
        self.store(a, self.words.get(a).unwrap().add(self.words.get(b).unwrap()).add(y));
        self.store(d, self.words.get(d).unwrap().xor(self.words.get(a).unwrap()).rotate_right(8));
        self.store(c, self.words.get(c).unwrap().add(self.words.get(d).unwrap()));
        self.store(b, self.words.get(b).unwrap().xor(self.words.get(c).unwrap()).rotate_right(7));
    }
}

impl Blake3Output {
    flow compress(self, root: bool) -> Array<Word32> ![] {
        let flags = self.flags + if root { 8 } else { 0 };
        var state = Blake3State { words= self.chaining.extend([
            BLAKE3_IV.get(0).unwrap(), BLAKE3_IV.get(1).unwrap(),
            BLAKE3_IV.get(2).unwrap(), BLAKE3_IV.get(3).unwrap(),
            Word32 { raw= self.counter % 4294967296 },
            Word32 { raw= self.counter / 4294967296 },
            Word32 { raw= self.length }, Word32 { raw= flags },
        ]) };
        for schedule in BLAKE3_SCHEDULE {
            state.mix(0, 4, 8, 12, self.block.get(schedule.get(0).unwrap()).unwrap(), self.block.get(schedule.get(1).unwrap()).unwrap());
            state.mix(1, 5, 9, 13, self.block.get(schedule.get(2).unwrap()).unwrap(), self.block.get(schedule.get(3).unwrap()).unwrap());
            state.mix(2, 6, 10, 14, self.block.get(schedule.get(4).unwrap()).unwrap(), self.block.get(schedule.get(5).unwrap()).unwrap());
            state.mix(3, 7, 11, 15, self.block.get(schedule.get(6).unwrap()).unwrap(), self.block.get(schedule.get(7).unwrap()).unwrap());
            state.mix(0, 5, 10, 15, self.block.get(schedule.get(8).unwrap()).unwrap(), self.block.get(schedule.get(9).unwrap()).unwrap());
            state.mix(1, 6, 11, 12, self.block.get(schedule.get(10).unwrap()).unwrap(), self.block.get(schedule.get(11).unwrap()).unwrap());
            state.mix(2, 7, 8, 13, self.block.get(schedule.get(12).unwrap()).unwrap(), self.block.get(schedule.get(13).unwrap()).unwrap());
            state.mix(3, 4, 9, 14, self.block.get(schedule.get(14).unwrap()).unwrap(), self.block.get(schedule.get(15).unwrap()).unwrap());
        }
        var output: Array<Word32> = [];
        var index: usize = 0;
        while index < 8 {
            output = output.push(state.words.get(index).unwrap().xor(state.words.get(index + 8).unwrap()));
            index = index + 1;
        }
        return output;
    }
}

impl Blake3 {
    flow block(input: bytes, offset: usize, length: usize) -> Array<Word32> ![] {
        var words: Array<Word32> = [];
        var word: usize = 0;
        while word < 16 {
            var raw: u128 = 0;
            var byte: usize = 0;
            var place: u128 = 1;
            while byte < 4 {
                let at = word * 4 + byte;
                if at < length { raw = raw + input.get(offset + at).unwrap().bits * place; }
                place = place * 256;
                byte = byte + 1;
            }
            words = words.push(Word32 { raw= raw });
            word = word + 1;
        }
        return words;
    }

    flow chunk(input: bytes, offset: usize, length: usize, counter: u128, key: Array<Word32>, flags: u128) -> Blake3Output ![] {
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

    flow parent(left: Array<Word32>, right: Array<Word32>, key: Array<Word32>, flags: u128) -> Blake3Output ![] {
        return Blake3Output { chaining= key, block= left + right, counter= 0, length= 64, flags= flags + 4 };
    }

    flow hash_internal(input: bytes, key: Array<Word32>, flags: u128) -> bytes ![] {
        var stack: List<Array<Word32>> = [];
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
                u8 { bits= word.raw % 256 }, u8 { bits= (word.raw / 256) % 256 },
                u8 { bits= (word.raw / 65536) % 256 }, u8 { bits= word.raw / 16777216 },
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
        var key: Array<Word32> = [];
        var index: usize = 0;
        while index < 8 {
            key = key.push(words.get(index).unwrap());
            index = index + 1;
        }
        return Blake3.hash_internal(input, key, 64);
    }
}
