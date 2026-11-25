module std.collections;

impl Iter<T> {
    public flow collect(self) -> Array<T> ![] {
        var output: Array<T> = [];
        for value in self { output = output.push(value); }
        return output;
    }
    public flow to_array(self) -> Array<T> ![] {
        var output: Array<T> = [];
        for value in self { output = output.push(value); }
        return output;
    }
    public flow to_list(self) -> List<T> ![] {
        var reversed: List<T> = [];
        for value in self { reversed = value :: reversed; }
        return reversed.reverse();
    }
    public flow count(self) -> usize ![] {
        var count: usize = 0;
        for _ in self { count = count + 1; }
        return count;
    }
    public flow nth<I ~ Index>(self, index: I) -> Option<T> ![] {
        let offset = match index.to_usize() { None => { return None; }, Some(value) => value };
        var ordinal: usize = 0;
        for value in self {
            if ordinal == offset { return Some(value); }
            ordinal = ordinal + 1;
        }
        return None;
    }
    public flow last(self) -> Option<T> ![] {
        var last: Option<T> = None;
        for value in self { last = Some(value); }
        return last;
    }
    public flow for_each<effect E>(self, f: (T) -> unit ![E]) -> unit ![E] {
        for value in self { f(value); }
    }
    public flow map_collect<U, effect E>(self, f: (T) -> U ![E]) -> Array<U> ![E] {
        var output: Array<U> = [];
        for value in self { output = output.push(f(value)); }
        return output;
    }
    public flow filter_collect<effect E>(self, predicate: (T) -> bool ![E]) -> Array<T> ![E] {
        var output: Array<T> = [];
        for value in self {
            if predicate(value) { output = output.push(value); }
        }
        return output;
    }
    public flow fold<A, effect E>(self, initial: A, f: (A, T) -> A ![E]) -> A ![E] {
        var accumulator = initial;
        for value in self { accumulator = f(accumulator, value); }
        return accumulator;
    }
    public flow reduce<effect E>(self, f: (T, T) -> T ![E]) -> Option<T> ![E] {
        let (first, remaining) = match (self.next)() { None => { return None; }, Some(pair) => pair };
        var accumulator = first;
        for value in remaining { accumulator = f(accumulator, value); }
        return Some(accumulator);
    }
    public flow any<effect E>(self, predicate: (T) -> bool ![E]) -> bool ![E] {
        for value in self { if predicate(value) { return true; } }
        return false;
    }
    public flow all<effect E>(self, predicate: (T) -> bool ![E]) -> bool ![E] {
        for value in self { if !predicate(value) { return false; } }
        return true;
    }
    public flow find<effect E>(self, predicate: (T) -> bool ![E]) -> Option<T> ![E] {
        for value in self { if predicate(value) { return Some(value); } }
        return None;
    }
    public flow position<effect E>(self, predicate: (T) -> bool ![E]) -> Option<usize> ![E] {
        var ordinal: usize = 0;
        for value in self {
            if predicate(value) { return Some(ordinal); }
            ordinal = ordinal + 1;
        }
        return None;
    }
    public flow min_by(self, compare: (T, T) -> i32 ![]) -> Option<T> ![] {
        var best: Option<T> = None;
        for value in self {
            best = match best { None => Some(value), Some(current) => if compare(value, current) < 0 { Some(value) } else { best } };
        }
        return best;
    }
    public flow max_by(self, compare: (T, T) -> i32 ![]) -> Option<T> ![] {
        var best: Option<T> = None;
        for value in self {
            best = match best { None => Some(value), Some(current) => if compare(value, current) > 0 { Some(value) } else { best } };
        }
        return best;
    }
    public flow min_by_key<K ~ Order>(self, key: (T) -> K ![]) -> Option<T> ![] {
        var best: Option<(T, K)> = None;
        for value in self {
            let selected = key(value);
            best = match best { None => Some((value, selected)), Some((old, old_key)) => if selected.compare(old_key) < 0 { Some((value, selected)) } else { best } };
        }
        return match best { None => None, Some((value, _)) => Some(value) };
    }
    public flow max_by_key<K ~ Order>(self, key: (T) -> K ![]) -> Option<T> ![] {
        var best: Option<(T, K)> = None;
        for value in self {
            let selected = key(value);
            best = match best { None => Some((value, selected)), Some((old, old_key)) => if selected.compare(old_key) > 0 { Some((value, selected)) } else { best } };
        }
        return match best { None => None, Some((value, _)) => Some(value) };
    }
}
impl Iter<T ~ Order> {
    public flow min(self) -> Option<T> ![] {
        var best: Option<T> = None;
        for value in self {
            best = match best { None => Some(value), Some(current) => if value.compare(current) < 0 { Some(value) } else { best } };
        }
        return best;
    }
    public flow max(self) -> Option<T> ![] {
        var best: Option<T> = None;
        for value in self {
            best = match best { None => Some(value), Some(current) => if value.compare(current) > 0 { Some(value) } else { best } };
        }
        return best;
    }
}
impl Iter<T ~ std.math.Number> {
    public flow sum(self) -> T ![] {
        var total: T = 0;
        for value in self { total = total + value; }
        return total;
    }
    public flow product(self) -> T ![] {
        var total: T = 1;
        for value in self { total = total * value; }
        return total;
    }
}
impl Iter<(A, B)> {
    public flow unzip(self) -> (Array<A>, Array<B>) ![] {
        var left: Array<A> = [];
        var right: Array<B> = [];
        for (a, b) in self { left = left.push(a); right = right.push(b); }
        return (left, right);
    }
}
