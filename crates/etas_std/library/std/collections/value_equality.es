module std.collections;

// The language's fixed collection equality uses these source-owned bodies.
// Eq remains evidence for the existing equality; users cannot overload ==.
impl Map<K ~ Hash, V> {
    flow __value_eq(self, other: Map<K, V>) -> bool ![] {
        if self.len() != other.len() { return false; }
        for (key, value) in self {
            match other.get(key) {
                None => { return false; },
                Some(found) => { if value != found { return false; } },
            };
        }
        return true;
    }
}

impl Set<T ~ Hash> {
    flow __value_eq(self, other: Set<T>) -> bool ![] {
        if self.len() != other.len() { return false; }
        for value in self {
            if !other.contains(value) { return false; }
        }
        return true;
    }
}

impl Slice<T> {
    flow __value_eq(self, other: Slice<T>) -> bool ![] {
        if self.len() != other.len() { return false; }
        var index: usize = 0;
        while index < self.len() {
            if self.get(index).unwrap() != other.get(index).unwrap() { return false; }
            index = index + 1;
        }
        return true;
    }
}

impl Stack<T> {
    flow __value_eq(self, other: Stack<T>) -> bool ![] {
        if self.len() != other.len() { return false; }
        for (left, right) in self.iter().zip(other.iter()) {
            if left != right { return false; }
        }
        return true;
    }
}

impl Queue<T> {
    flow __value_eq(self, other: Queue<T>) -> bool ![] {
        if self.len() != other.len() { return false; }
        for (left, right) in self.iter().zip(other.iter()) {
            if left != right { return false; }
        }
        return true;
    }
}

impl Deque<T> {
    flow __value_eq(self, other: Deque<T>) -> bool ![] {
        if self.len() != other.len() { return false; }
        for (left, right) in self.iter().zip(other.iter()) {
            if left != right { return false; }
        }
        return true;
    }
}

impl BoundedDeque<T> {
    flow __value_eq(self, other: BoundedDeque<T>) -> bool ![] {
        if self.maximum != other.maximum || self.overflow != other.overflow { return false; }
        if self.len() != other.len() { return false; }
        for (left, right) in self.iter().zip(other.iter()) {
            if left != right { return false; }
        }
        return true;
    }
}
