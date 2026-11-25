module std.collections;

public type BitSet = { private size: usize, private words: Array<u64> };
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
                if bits % 2 != 0 { count = count + 1; }
                bits = bits / 2;
            }
        }
        return count;
    }
    public flow is_empty(self) -> bool ![] {
        for word in self.words { if word != 0 { return false; } }
        return true;
    }
    flow mask(bit: usize) -> u128 ![] {
        var mask: u128 = 1;
        var offset: usize = 0;
        while offset < bit { mask = mask * 2; offset = offset + 1; }
        return mask;
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
            var left = self.words.get(index).unwrap().bits;
            var right = other.words.get(index).unwrap().bits;
            var output: u128 = 0;
            var place: u128 = 1;
            var bit: usize = 0;
            while bit < 64 {
                let a = left % 2;
                let b = right % 2;
                let selected = match operation {
                    BitOperation.Union => a == 1 || b == 1,
                    BitOperation.Intersection => a == 1 && b == 1,
                    BitOperation.Difference => a == 1 && b == 0,
                    BitOperation.SymmetricDifference => a != b,
                };
                if selected { output = output + place; }
                left = left / 2; right = right / 2; place = place * 2; bit = bit + 1;
            }
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
            var a = self.words.get(index).unwrap().bits;
            var b = other.words.get(index).unwrap().bits;
            var bit: usize = 0;
            while bit < 64 {
                if a % 2 == 1 && b % 2 == 0 { return false; }
                a = a / 2; b = b / 2; bit = bit + 1;
            }
            index = index + 1;
        }
        return true;
    }
    public flow is_disjoint(self, other: BitSet) -> bool ![Error<CollectionError>] {
        if self.size != other.size { perform Error.raise<CollectionError>(CollectionError.IncompatibleBounds); }
        var index: usize = 0;
        while index < self.words.length {
            var a = self.words.get(index).unwrap().bits;
            var b = other.words.get(index).unwrap().bits;
            var bit: usize = 0;
            while bit < 64 {
                if a % 2 == 1 && b % 2 == 1 { return false; }
                a = a / 2; b = b / 2; bit = bit + 1;
            }
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
