module std.collections;

public import std.collections.OverflowPolicy.{Reject, DropFront, DropBack};

public enum OverflowPolicy { Reject, DropFront, DropBack }
public type BoundedDeque<T> = { private values: Deque<T>, private maximum: usize, private overflow: OverflowPolicy };

impl BoundedDeque<T> {
    public flow new(capacity: usize, overflow: OverflowPolicy = Reject) -> BoundedDeque<T> ![Error<CollectionError>] {
        if capacity == 0 { perform Error.raise<CollectionError>(CollectionError.InvalidCapacity(capacity)); }
        return BoundedDeque { values= Deque.new(), maximum= capacity, overflow= overflow };
    }
    public flow push_front(self, value: T) -> (BoundedDeque<T>, Option<T>) ![Error<CollectionError>] {
        var values = self.values;
        var evicted: Option<T> = None;
        if values.len() == self.maximum {
            let removed = match self.overflow {
                OverflowPolicy.Reject => perform Error.raise<CollectionError>(CollectionError.Full),
                OverflowPolicy.DropFront => values.pop_front(),
                OverflowPolicy.DropBack => values.pop_back(),
            };
            let (tail, old) = removed; values = tail; evicted = old;
        }
        return (BoundedDeque { values= values.push_front(value), maximum= self.maximum, overflow= self.overflow }, evicted);
    }
    public flow push_back(self, value: T) -> (BoundedDeque<T>, Option<T>) ![Error<CollectionError>] {
        var values = self.values;
        var evicted: Option<T> = None;
        if values.len() == self.maximum {
            let removed = match self.overflow {
                OverflowPolicy.Reject => perform Error.raise<CollectionError>(CollectionError.Full),
                OverflowPolicy.DropFront => values.pop_front(),
                OverflowPolicy.DropBack => values.pop_back(),
            };
            let (tail, old) = removed; values = tail; evicted = old;
        }
        return (BoundedDeque { values= values.push_back(value), maximum= self.maximum, overflow= self.overflow }, evicted);
    }
    public flow pop_front(self) -> (BoundedDeque<T>, Option<T>) ![] {
        let (values, value) = self.values.pop_front();
        return (BoundedDeque { values= values, maximum= self.maximum, overflow= self.overflow }, value);
    }
    public flow pop_back(self) -> (BoundedDeque<T>, Option<T>) ![] {
        let (values, value) = self.values.pop_back();
        return (BoundedDeque { values= values, maximum= self.maximum, overflow= self.overflow }, value);
    }
    public flow peek_front(self) -> Option<T> ![] { return self.values.peek_front(); }
    public flow peek_back(self) -> Option<T> ![] { return self.values.peek_back(); }
    public flow len(self) -> usize ![] { return self.values.len(); }
    public flow capacity(self) -> usize ![] { return self.maximum; }
    public flow is_empty(self) -> bool ![] { return self.values.is_empty(); }
    public flow is_full(self) -> bool ![] { return self.values.len() == self.maximum; }
    public flow clear(self) -> BoundedDeque<T> ![] {
        return BoundedDeque { values= Deque.new(), maximum= self.maximum, overflow= self.overflow };
    }
}
impl BoundedDeque<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return self.values.iter();
    }
}
