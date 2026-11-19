module std.collections;

public spec Iterable<T> {
    flow iter(self) -> Iter<T> ![];
}

impl Iter<T> {
    public flow empty() -> Iter<T> ![] {
        return Iter { next= () => None };
    }
}

impl Iter<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return self;
    }
}

impl Array<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => match self.get(0) {
            Some(value) => Some((value, self.iter_from(1))),
            None => None,
        } };
    }
}

impl Array<T> {
    flow iter_from(self, index: usize) -> Iter<T> ![] {
        return Iter { next= () => match self.get(index) {
            Some(value) => Some((value, self.iter_from(index + 1))),
            None => None,
        } };
    }
}

impl List<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => match self {
            [] => None,
            head :: tail => Some((head, tail.iter())),
        } };
    }
}

impl Map<K, V> ~ Iterable<(K, V)> {
    public flow iter(self) -> Iter<(K, V)> ![] {
        return Iter { next= () => match self.entries {
            [] => None,
            entry :: tail => Some((entry, tail.iter())),
        } };
    }
}

impl Set<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => match self.elements {
            [] => None,
            value :: tail => Some((value, tail.iter())),
        } };
    }
}

impl Slice<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => {
            if self.length == 0 {
                return None;
            }
            let remaining = Slice {
                source= self.source,
                start= self.start + 1,
                length= self.length - 1,
            };
            return Some((self.source.get(self.start).unwrap(), remaining.iter()));
        } };
    }
}

impl Stack<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => match self.items {
            [] => None,
            value :: tail => Some((value, tail.iter())),
        } };
    }
}

impl Queue<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => {
            let (remaining, value) = self.pop();
            return match value {
                None => None,
                Some(value) => Some((value, remaining.iter())),
            };
        } };
    }
}

impl Deque<T> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] {
        return Iter { next= () => {
            let (remaining, value) = self.pop_front();
            return match value {
                None => None,
                Some(value) => Some((value, remaining.iter())),
            };
        } };
    }
}

impl bytes ~ Iterable<u8> {
    public flow iter(self) -> Iter<u8> ![] {
        return Iter { next= () => match self.get(0) {
            None => None,
            Some(value) => Some((value, self.iter_from(1))),
        } };
    }
}

impl bytes {
    flow iter_from(self, offset: usize) -> Iter<u8> ![] {
        return Iter { next= () => match self.get(offset) {
            None => None,
            Some(value) => Some((value, self.iter_from(offset + 1))),
        } };
    }
}

impl Iter<T> {
    public flow map<U>(self, transform: (T) -> U ![]) -> Iter<U> ![] {
        return Iter { next= () => match (self.next)() {
            None => None,
            Some((value, remaining)) => Some((transform(value), remaining.map(transform))),
        } };
    }
}
