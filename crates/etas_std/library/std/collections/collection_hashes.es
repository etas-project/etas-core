module std.collections;

impl Array<T ~ Eq> ~ Eq {}
impl Array<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = state.write("std.collections.Array".utf8);
        output = self.len().hash(output);
        for value in self { output = value.hash(output); }
        return output;
    }
}
impl List<T ~ Eq> ~ Eq {}
impl List<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = state.write("std.collections.List".utf8);
        output = self.len().hash(output);
        for value in self { output = value.hash(output); }
        return output;
    }
}
impl Slice<T ~ Eq> ~ Eq {}
impl Slice<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = state.write("std.collections.Slice".utf8);
        output = self.len().hash(output);
        for value in self { output = value.hash(output); }
        return output;
    }
}
impl Stack<T ~ Eq> ~ Eq {}
impl Stack<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = state.write("std.collections.Stack".utf8);
        output = self.len().hash(output);
        for value in self { output = value.hash(output); }
        return output;
    }
}
impl Queue<T ~ Eq> ~ Eq {}
impl Queue<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = state.write("std.collections.Queue".utf8);
        output = self.len().hash(output);
        for value in self { output = value.hash(output); }
        return output;
    }
}
impl Deque<T ~ Eq> ~ Eq {}
impl Deque<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = state.write("std.collections.Deque".utf8);
        output = self.len().hash(output);
        for value in self { output = value.hash(output); }
        return output;
    }
}

impl Map<K ~ Hash, V ~ Eq> ~ Eq {}
impl Map<K ~ Hash, V ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var combined: u128 = 0;
        for (key, value) in self {
            let key_state = key.hash(Hasher.new());
            let digest = value.hash(key_state).finish();
            combined = (combined + digest.bits) % 18446744073709551616;
        }
        let framed = self.len().hash(state.write("std.collections.Map".utf8));
        return u64 { bits= combined }.hash(framed);
    }
}
impl Set<T ~ Hash> ~ Eq {}
impl Set<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var combined: u128 = 0;
        for value in self {
            combined = (combined + value.hash(Hasher.new()).finish().bits) % 18446744073709551616;
        }
        let framed = self.len().hash(state.write("std.collections.Set".utf8));
        return u64 { bits= combined }.hash(framed);
    }
}
impl BitSet ~ Eq {}
impl BitSet ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = self.universe_len().hash(state.write("std.collections.BitSet".utf8));
        for index in self { output = index.hash(output); }
        return output;
    }
}
impl OverflowPolicy ~ Eq {}
impl OverflowPolicy ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        let tag: u8 = match self { OverflowPolicy.Reject => 0, OverflowPolicy.DropFront => 1, OverflowPolicy.DropBack => 2 };
        return tag.hash(state);
    }
}
impl BoundedDeque<T ~ Eq> ~ Eq {}
impl BoundedDeque<T ~ Hash> ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var output = self.maximum.hash(state.write("std.collections.BoundedDeque".utf8));
        output = self.overflow.hash(output);
        output = self.values.len().hash(output);
        for value in self { output = value.hash(output); }
        return output;
    }
}
