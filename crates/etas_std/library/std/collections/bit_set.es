module std.collections;

public type BitSet = { private size: usize, private words: Array<u64> };
let BYTE_POPCOUNTS: Array<usize> = [0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7, 1, 2, 2, 3, 2, 3, 3, 4, 2, 3, 3, 4, 3, 4, 4, 5, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7, 2, 3, 3, 4, 3, 4, 4, 5, 3, 4, 4, 5, 4, 5, 5, 6, 3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7, 3, 4, 4, 5, 4, 5, 5, 6, 4, 5, 5, 6, 5, 6, 6, 7, 4, 5, 5, 6, 5, 6, 6, 7, 5, 6, 6, 7, 6, 7, 7, 8];
let BIT_MASKS: Array<u128> = [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576, 2097152, 4194304, 8388608, 16777216, 33554432, 67108864, 134217728, 268435456, 536870912, 1073741824, 2147483648, 4294967296, 8589934592, 17179869184, 34359738368, 68719476736, 137438953472, 274877906944, 549755813888, 1099511627776, 2199023255552, 4398046511104, 8796093022208, 17592186044416, 35184372088832, 70368744177664, 140737488355328, 281474976710656, 562949953421312, 1125899906842624, 2251799813685248, 4503599627370496, 9007199254740992, 18014398509481984, 36028797018963968, 72057594037927936, 144115188075855872, 288230376151711744, 576460752303423488, 1152921504606846976, 2305843009213693952, 4611686018427387904, 9223372036854775808];
enum BitOperation { Union, Intersection, Difference, SymmetricDifference }

impl BitSet {
    public flow new(size: usize) -> BitSet ![] {
        return BitSet { size= size, words= Array.repeat(size / 64 + if size % 64 == 0 { 0 } else { 1 }, 0) };
    }
    public flow from_indices(size: usize, indices: Iter<usize>) -> BitSet ![Error<std.runtime.error.IndexError>] {
        var output = BitSet.new(size);
        for index in indices { output = output.insert(index); }
        return output;
    }
    public flow universe_len(self) -> usize ![] { return self.size; }
    public flow count(self) -> usize ![] {
        var count: usize = 0;
        for word in self.words {
            var bits = word.bits;
            while bits != 0 {
                count = count + BYTE_POPCOUNTS.get(usize { bits= bits % 256 }).unwrap();
                bits = bits / 256;
            }
        }
        return count;
    }
    public flow is_empty(self) -> bool ![] {
        for word in self.words { if word != 0 { return false; } }
        return true;
    }
    flow mask(bit: usize) -> u128 ![] {
        return BIT_MASKS.get(bit).unwrap();
    }
    public flow contains<I ~ Index>(self, index: I) -> bool ![] {
        let offset = match index.to_usize() { None => { return false; }, Some(value) => value };
        if offset >= self.size { return false; }
        return (self.words.get(offset / 64).unwrap().bits / BitSet.mask(offset % 64)) % 2 == 1;
    }
    public flow insert<I ~ Index>(self, index: I) -> BitSet ![Error<std.runtime.error.IndexError>] {
        let offset = match index.to_usize() { None => usize.maximum(), Some(value) => value };
        if offset >= self.size { perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= offset, length= self.size }); }
        let mask = BitSet.mask(offset % 64);
        let old = self.words.get(offset / 64).unwrap().bits;
        if (old / mask) % 2 == 1 { return self; }
        let words = self.words.set(offset / 64, u64 { bits= old + mask })?.unwrap();
        return BitSet { size= self.size, words= words };
    }
    public flow remove<I ~ Index>(self, index: I) -> BitSet ![Error<std.runtime.error.IndexError>] {
        let offset = match index.to_usize() { None => usize.maximum(), Some(value) => value };
        if offset >= self.size { perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= offset, length= self.size }); }
        let mask = BitSet.mask(offset % 64);
        let old = self.words.get(offset / 64).unwrap().bits;
        if (old / mask) % 2 == 0 { return self; }
        let words = self.words.set(offset / 64, u64 { bits= old - mask })?.unwrap();
        return BitSet { size= self.size, words= words };
    }
    flow combine(self, other: BitSet, operation: BitOperation) -> BitSet ![Error<CollectionError>] {
        if self.size != other.size { perform Error.raise<CollectionError>(CollectionError.IncompatibleBounds); }
        var words: Array<u64> = [];
        var index: usize = 0;
        while index < self.words.length {
            let left = self.words.get(index).unwrap().bits;
            let right = other.words.get(index).unwrap().bits;
            let different = left ^ right;
            // u64 lanes widened to u128: the sum cannot overflow.
            let common = (left + right - different) / 2;
            let output = match operation {
                BitOperation.Union => common + different,
                BitOperation.Intersection => common,
                BitOperation.Difference => left - common,
                BitOperation.SymmetricDifference => different,
            };
            words = words.push(u64 { bits= output });
            index = index + 1;
        }
        return BitSet { size= self.size, words= words };
    }
    public flow union(self, other: BitSet) -> BitSet ![Error<CollectionError>] {
        return self.combine(other, BitOperation.Union);
    }
    public flow intersection(self, other: BitSet) -> BitSet ![Error<CollectionError>] {
        return self.combine(other, BitOperation.Intersection);
    }
    public flow difference(self, other: BitSet) -> BitSet ![Error<CollectionError>] {
        return self.combine(other, BitOperation.Difference);
    }
    public flow symmetric_difference(self, other: BitSet) -> BitSet ![Error<CollectionError>] {
        return self.combine(other, BitOperation.SymmetricDifference);
    }
    public flow is_subset(self, other: BitSet) -> bool ![Error<CollectionError>] {
        if self.size != other.size { perform Error.raise<CollectionError>(CollectionError.IncompatibleBounds); }
        var index: usize = 0;
        while index < self.words.length {
            let a = self.words.get(index).unwrap().bits;
            let b = other.words.get(index).unwrap().bits;
            if (a + b - (a ^ b)) / 2 != a { return false; }
            index = index + 1;
        }
        return true;
    }
    public flow is_disjoint(self, other: BitSet) -> bool ![Error<CollectionError>] {
        if self.size != other.size { perform Error.raise<CollectionError>(CollectionError.IncompatibleBounds); }
        var index: usize = 0;
        while index < self.words.length {
            let a = self.words.get(index).unwrap().bits;
            let b = other.words.get(index).unwrap().bits;
            if a + b != (a ^ b) { return false; }
            index = index + 1;
        }
        return true;
    }
    public flow complement(self) -> BitSet ![] {
        var words: Array<u64> = [];
        var index: usize = 0;
        for word in self.words {
            var mask: u128 = 18446744073709551615;
            if index + 1 == self.words.length && self.size % 64 != 0 { mask = BitSet.mask(self.size % 64) - 1; }
            words = words.push(u64 { bits= mask - word.bits });
            index = index + 1;
        }
        return BitSet { size= self.size, words= words };
    }
    public flow clear(self) -> BitSet ![] {
        return BitSet { size= self.size, words= Array.repeat(self.words.length, 0) };
    }
    flow cursor(self, word_index: usize, bit_index: usize) -> Iter<usize> ![] {
        return Iter { next= () => {
            var word = word_index;
            var bit = bit_index;
            while word < self.words.length {
                var bits = self.words.get(word).unwrap().bits / BitSet.mask(bit);
                if bits == 0 { word = word + 1; bit = 0; continue; }
                while bits % 2 == 0 { bits = bits / 2; bit = bit + 1; }
                let value = word * 64 + bit;
                let tail = if bit == 63 { self.cursor(word + 1, 0) } else { self.cursor(word, bit + 1) };
                return Some((value, tail));
            }
            return None;
        } };
    }
}
impl BitSet ~ Iterable<usize> {
    public flow iter(self) -> Iter<usize> ![] {
        return self.cursor(0, 0);
    }
}
