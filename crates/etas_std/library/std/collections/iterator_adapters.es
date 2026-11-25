module std.collections;

impl Iter<T> {
    public flow once(value: T) -> Iter<T> ![] {
        return Iter { next= () => Some((value, Iter.empty())) };
    }
    public flow repeat_n(value: T, count: usize) -> Iter<T> ![] {
        return Iter { next= () => {
            if count == 0 { return None; }
            return Some((value, Iter.repeat_n(value, count - 1)));
        } };
    }
    public flow init(count: usize, f: (usize) -> T ![]) -> Iter<T> ![] {
        return Iter.init_from(0, count, f);
    }
    flow init_from(index: usize, count: usize, f: (usize) -> T ![]) -> Iter<T> ![] {
        return Iter { next= () => {
            if index == count { return None; }
            return Some((f(index), Iter.init_from(index + 1, count, f)));
        } };
    }
    public flow unfold<S>(state: S, step: (S) -> Option<(T, S)> ![]) -> Iter<T> ![] {
        return Iter { next= () => match step(state) {
            None => None,
            Some((value, remaining)) => Some((value, Iter.unfold(remaining, step))),
        } };
    }
    public flow peek(self) -> Option<T> ![] {
        return match (self.next)() { None => None, Some((value, _)) => Some(value) };
    }
    public flow scan<A, U>(self, initial: A, step: (A, T) -> (A, U) ![]) -> Iter<U> ![] {
        return Iter { next= () => match (self.next)() {
            None => None,
            Some((value, remaining)) => {
                let (state, output) = step(initial, value);
                Some((output, remaining.scan(state, step)))
            },
        } };
    }
    public flow filter(self, predicate: (T) -> bool ![]) -> Iter<T> ![] {
        return Iter { next= () => {
            var cursor = self;
            while true {
                match (cursor.next)() {
                    None => { return None; },
                    Some((value, tail)) => {
                        if predicate(value) { return Some((value, tail.filter(predicate))); }
                        cursor = tail;
                    },
                };
            }
        } };
    }
    public flow filter_map<U>(self, f: (T) -> Option<U> ![]) -> Iter<U> ![] {
        return Iter { next= () => {
            var cursor = self;
            while true {
                match (cursor.next)() {
                    None => { return None; },
                    Some((value, tail)) => {
                        match f(value) {
                            Some(output) => { return Some((output, tail.filter_map(f))); },
                            None => {},
                        };
                        cursor = tail;
                    },
                };
            }
        } };
    }
    public flow flat_map<U>(self, f: (T) -> Iter<U> ![]) -> Iter<U> ![] {
        return Iter { next= () => {
            var outer = self;
            while true {
                match (outer.next)() {
                    None => { return None; },
                    Some((value, tail)) => {
                        match ((f(value)).next)() {
                            Some((head, inner)) => { return Some((head, inner.chain(tail.flat_map(f)))); },
                            None => { outer = tail; },
                        };
                    },
                };
            }
        } };
    }
    public flow chain(self, other: Iter<T>) -> Iter<T> ![] {
        return Iter { next= () => match (self.next)() {
            None => (other.next)(),
            Some((head, tail)) => Some((head, tail.chain(other))),
        } };
    }
    public flow zip<U>(self, other: Iter<U>) -> Iter<(T, U)> ![] {
        return Iter { next= () => match (self.next)() {
            None => None,
            Some((left, left_tail)) => match (other.next)() {
                None => None,
                Some((right, right_tail)) => Some(((left, right), left_tail.zip(right_tail))),
            },
        } };
    }
    public flow enumerate(self) -> Iter<(usize, T)> ![] {
        return self.enumerate_from(0);
    }
    flow enumerate_from(self, ordinal: usize) -> Iter<(usize, T)> ![] {
        return Iter { next= () => match (self.next)() {
            None => None,
            Some((value, remaining)) => Some(((ordinal, value), remaining.enumerate_from(ordinal + 1))),
        } };
    }
    public flow take(self, count: usize) -> Iter<T> ![] {
        return Iter { next= () => {
            if count == 0 { return None; }
            return match (self.next)() {
                None => None,
                Some((head, tail)) => Some((head, tail.take(count - 1))),
            };
        } };
    }
    public flow skip(self, count: usize) -> Iter<T> ![] {
        return Iter { next= () => {
            var cursor = self;
            var remaining = count;
            while remaining > 0 {
                match (cursor.next)() {
                    None => { return None; },
                    Some((_, tail)) => { cursor = tail; },
                };
                remaining = remaining - 1;
            }
            return (cursor.next)();
        } };
    }
    public flow take_while(self, predicate: (T) -> bool ![]) -> Iter<T> ![] {
        return Iter { next= () => match (self.next)() {
            None => None,
            Some((head, tail)) => if predicate(head) { Some((head, tail.take_while(predicate))) } else { None },
        } };
    }
    public flow skip_while(self, predicate: (T) -> bool ![]) -> Iter<T> ![] {
        return Iter { next= () => {
            var cursor = self;
            while true {
                match (cursor.next)() {
                    None => { return None; },
                    Some((head, tail)) => {
                        if !predicate(head) { return Some((head, tail)); }
                        cursor = tail;
                    },
                };
            }
        } };
    }
    public flow chunks(self, size: usize) -> Iter<Array<T>> ![Error<CollectionError>] {
        if size == 0 { perform Error.raise<CollectionError>(CollectionError.InvalidChunkSize(size)); }
        return self.chunk_cursor(size);
    }
    flow chunk_cursor(self, size: usize) -> Iter<Array<T>> ![] {
        return Iter { next= () => {
            var output: Array<T> = [];
            var cursor = self;
            while output.length < size {
                match (cursor.next)() {
                    None => { break; },
                    Some((value, tail)) => { output = output.push(value); cursor = tail; },
                };
            }
            if output.length == 0 { return None; }
            return Some((output, cursor.chunk_cursor(size)));
        } };
    }
    public flow windows(self, size: usize) -> Iter<Array<T>> ![Error<CollectionError>] {
        if size == 0 { perform Error.raise<CollectionError>(CollectionError.InvalidWindowSize(size)); }
        return self.window_cursor(size);
    }
    flow window_cursor(self, size: usize) -> Iter<Array<T>> ![] {
        return Iter { next= () => {
            var output: Array<T> = [];
            var cursor = self;
            while output.length < size {
                match (cursor.next)() {
                    None => { return None; },
                    Some((value, tail)) => { output = output.push(value); cursor = tail; },
                };
            }
            return Some((output, self.skip(1).window_cursor(size)));
        } };
    }
    public flow step_by(self, step: usize) -> Iter<T> ![Error<CollectionError>] {
        if step == 0 { perform Error.raise<CollectionError>(CollectionError.InvalidStep(step)); }
        return self.stepped(step);
    }
    flow stepped(self, step: usize) -> Iter<T> ![] {
        return Iter { next= () => match (self.next)() {
            None => None,
            Some((head, tail)) => Some((head, tail.skip(step - 1).stepped(step))),
        } };
    }
}
impl Iter<Iter<U>> {
    public flow flatten(self) -> Iter<U> ![] {
        return Iter { next= () => {
            var outer = self;
            while true {
                match (outer.next)() {
                    None => { return None; },
                    Some((inner, tail)) => match (inner.next)() {
                        None => { outer = tail; },
                        Some((head, rest)) => { return Some((head, rest.chain(tail.flatten()))); },
                    },
                };
            }
        } };
    }
}
