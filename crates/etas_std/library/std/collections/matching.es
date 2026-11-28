module std.collections;

public flow starts_with<T ~ Eq>(items: Iter<T>, prefix: Array<T>) -> bool ![] {
    var cursor = items;
    for expected in prefix {
        match (cursor.next)() {
            None => { return false; },
            Some((value, tail)) => {
                if value != expected { return false; }
                cursor = tail;
            },
        };
    }
    return true;
}

// The failure table records the longest proper prefix that is also a suffix.
// A mismatch can therefore reuse input that has already been read.
type SequencePattern<T> = { private values: Array<T>, private fallback: Array<usize> };

impl SequencePattern<T ~ Eq> {
    flow new(values: Array<T>) -> SequencePattern<T> ![] {
        var fallback: Array<usize> = [0];
        var matched: usize = 0;
        var i: usize = 1;
        while i < values.length {
            let value = values.get(i).unwrap();
            while matched > 0 && values.get(matched).unwrap() != value {
                matched = fallback.get(matched - 1).unwrap();
            }
            if values.get(matched).unwrap() == value { matched = matched + 1; }
            fallback = fallback.push(matched);
            i = i + 1;
        }
        return SequencePattern { values= values, fallback= fallback };
    }

    flow advance(self, matched: usize, value: T) -> usize ![] {
        var length = matched;
        while true {
            if self.values.get(length).unwrap() == value { return length + 1; }
            if length == 0 { return 0; }
            length = self.fallback.get(length - 1).unwrap();
        }
    }

    flow matches(self, items: Iter<T>, offset: usize, matched: usize, overlapping: bool) -> Iter<(usize, usize)> ![] {
        return Iter { next= () => {
            var cursor = items;
            var end = offset;
            var length = matched;
            while true {
                match (cursor.next)() {
                    None => { return None; },
                    Some((value, tail)) => {
                        end = end + 1;
                        length = self.advance(length, value);
                        if length == self.values.length {
                            let restart = if overlapping { self.fallback.get(length - 1).unwrap() } else { 0 };
                            return Some(((end - length, end), self.matches(tail, end, restart, overlapping)));
                        }
                        cursor = tail;
                    },
                };
            }
        } };
    }

    flow segments(self, items: Iter<T>) -> Iter<Array<T>> ![] {
        return Iter { next= () => {
            var cursor = items;
            var segment: Array<T> = [];
            var matched: usize = 0;
            while true {
                match (cursor.next)() {
                    None => { return Some((segment, Iter.empty())); },
                    Some((value, tail)) => {
                        segment = segment.push(value);
                        matched = self.advance(matched, value);
                        if matched == self.values.length {
                            let output = ArrayStorage.truncate(segment, segment.length - matched);
                            return Some((output, self.segments(tail)));
                        }
                        cursor = tail;
                    },
                };
            }
        } };
    }
}

impl Iter<T> {
    flow boundaries_after(self, offset: usize) -> Iter<(usize, usize)> ![] {
        return Iter { next= () => match (self.next)() {
            None => None,
            Some((_, tail)) => {
                let end = offset + 1;
                Some(((end, end), tail.boundaries_after(end)))
            },
        } };
    }

    flow singleton_segments(self) -> Iter<Array<T>> ![] {
        return Iter { next= () => match (self.next)() {
            None => Some(([], Iter.empty())),
            Some((value, tail)) => Some(([value], tail.singleton_segments())),
        } };
    }

    flow split_segments(self, predicate: (T) -> bool ![]) -> Iter<Array<T>> ![] {
        return Iter { next= () => {
            var cursor = self;
            var segment: Array<T> = [];
            while true {
                match (cursor.next)() {
                    None => { return Some((segment, Iter.empty())); },
                    Some((value, tail)) => {
                        if predicate(value) { return Some((segment, tail.split_segments(predicate))); }
                        segment = segment.push(value);
                        cursor = tail;
                    },
                };
            }
        } };
    }
}

public flow find_sequence<T ~ Eq>(items: Iter<T>, pattern: Array<T>) -> Option<usize> ![] {
    if pattern.length == 0 { return Some(0); }
    let compiled = SequencePattern.new(pattern);
    var end: usize = 0;
    var matched: usize = 0;
    for value in items {
        end = end + 1;
        matched = compiled.advance(matched, value);
        if matched == pattern.length { return Some(end - matched); }
    }
    return None;
}

public flow find_all_sequences<T ~ Eq>(items: Iter<T>, pattern: Array<T>, overlapping: bool = false) -> Iter<(usize, usize)> ![] {
    if pattern.length == 0 { return Iter { next= () => Some(((0, 0), items.boundaries_after(0))) }; }
    return SequencePattern.new(pattern).matches(items, 0, 0, overlapping);
}

public flow split_on<T ~ Eq>(items: Iter<T>, separator: T) -> Iter<Array<T>> ![] {
    return items.split_segments((value: T) => value == separator);
}

public flow split_when<T>(items: Iter<T>, predicate: (T) -> bool ![]) -> Iter<Array<T>> ![] {
    return items.split_segments(predicate);
}

public flow split_sequence<T ~ Eq>(items: Iter<T>, separator: Array<T>) -> Iter<Array<T>> ![] {
    if separator.length == 0 { return Iter { next= () => Some(([], items.singleton_segments())) }; }
    return SequencePattern.new(separator).segments(items);
}

public flow split_when_collect<T, effect E>(items: Iter<T>, predicate: (T) -> bool ![E]) -> Array<Array<T>> ![E] {
    var output: Array<Array<T>> = [];
    var segment: Array<T> = [];
    for value in items {
        if predicate(value) {
            output = output.push(segment);
            segment = [];
        } else {
            segment = segment.push(value);
        }
    }
    return output.push(segment);
}
