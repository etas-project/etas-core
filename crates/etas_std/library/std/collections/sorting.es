module std.collections;

type MergeSorter<T> = { private compare: (T, T) -> i32 ![] };
impl MergeSorter<T> {
    flow sorted(self, values: Array<T>) -> Array<T> ![] {
        if values.length < 2 { return values; }
        // Discover natural runs; reversing only strict descending runs preserves stability.
        var source = values;
        var runs: Array<usize> = [0];
        var start: usize = 0;
        while start < source.length {
            var end = start + 1;
            if end < source.length {
                let descending = (self.compare)(source.get(start).unwrap(), source.get(end).unwrap()) > 0;
                end = end + 1;
                while end < source.length {
                    let order = (self.compare)(source.get(end - 1).unwrap(), source.get(end).unwrap());
                    if (descending && order <= 0) || (!descending && order > 0) { break; }
                    end = end + 1;
                }
                if descending {
                    var left = start; var right = end;
                    while left + 1 < right {
                        right = right - 1;
                        let x = source.get(left).unwrap(); let y = source.get(right).unwrap();
                        source = source.set(left, y)?.unwrap().set(right, x)?.unwrap();
                        left = left + 1;
                    }
                }
            }
            runs = runs.push(end);
            start = end;
        }
        // Two full-size workspaces replace recursively allocated subarrays.
        var target = source;
        while runs.length > 2 {
            var next_runs: Array<usize> = [0];
            var run: usize = 0;
            while run + 1 < runs.length {
                let begin = runs.get(run).unwrap();
                let middle = runs.get(run + 1).unwrap();
                let end = if run + 2 < runs.length { runs.get(run + 2).unwrap() } else { middle };
                var a = begin; var b = middle; var out = begin;
                while a < middle && b < end {
                    let x = source.get(a).unwrap(); let y = source.get(b).unwrap();
                    if (self.compare)(x, y) <= 0 { target = target.set(out, x)?.unwrap(); a = a + 1; }
                    else { target = target.set(out, y)?.unwrap(); b = b + 1; }
                    out = out + 1;
                }
                while a < middle { target = target.set(out, source.get(a).unwrap())?.unwrap(); a = a + 1; out = out + 1; }
                while b < end { target = target.set(out, source.get(b).unwrap())?.unwrap(); b = b + 1; out = out + 1; }
                next_runs = next_runs.push(end);
                run = run + 2;
            }
            let previous = source; source = target; target = previous;
            runs = next_runs;
        }
        return source;
    }
}
public flow sort<T ~ Order>(values: Array<T>) -> Array<T> ![] {
    let sorter = MergeSorter { compare= (left: T, right: T) => left.compare(right) };
    return sorter.sorted(values);
}
public flow sort_by<T>(values: Array<T>, compare: (T, T) -> i32 ![]) -> Array<T> ![] {
    return MergeSorter { compare= compare }.sorted(values);
}
public flow stable_sort<T ~ Order>(values: Array<T>) -> Array<T> ![] {
    let sorter = MergeSorter { compare= (left: T, right: T) => left.compare(right) };
    return sorter.sorted(values);
}
public flow stable_sort_by<T>(values: Array<T>, compare: (T, T) -> i32 ![]) -> Array<T> ![] {
    return MergeSorter { compare= compare }.sorted(values);
}
public flow sort_by_key<T, K ~ Order>(values: Array<T>, key: (T) -> K ![]) -> Array<T> ![] {
    var keyed: Array<(K, T)> = [];
    for value in values { keyed = keyed.push((key(value), value)); }
    let sorter = MergeSorter { compare= (a: (K, T), b: (K, T)) => { let (left, _) = a; let (right, _) = b; return left.compare(right); } };
    var output: Array<T> = [];
    for (_, value) in sorter.sorted(keyed) { output = output.push(value); }
    return output;
}
public flow stable_sort_by_key<T, K ~ Order>(values: Array<T>, key: (T) -> K ![]) -> Array<T> ![] {
    var keyed: Array<(K, T)> = [];
    for value in values { keyed = keyed.push((key(value), value)); }
    let sorter = MergeSorter { compare= (a: (K, T), b: (K, T)) => { let (left, _) = a; let (right, _) = b; return left.compare(right); } };
    var output: Array<T> = [];
    for (_, value) in sorter.sorted(keyed) { output = output.push(value); }
    return output;
}
type SortedMerge<T> = { private compare: (T, T) -> i32 ![] };

impl SortedMerge<T> {
    // Hold the unchosen head across calls. Only advance the input that supplied the last item.
    flow cursor(self, left: Iter<T>, right: Iter<T>, a: Option<T>, b: Option<T>) -> Iter<T> ![] {
        return Iter { next= () => {
            let lhs = match a { Some(x) => Some((x, left)), None => (left.next)() };
            let rhs = match b { Some(y) => Some((y, right)), None => (right.next)() };
            return match (lhs, rhs) {
                (None, tail) => tail,
                (tail, None) => tail,
                (Some((x, xs)), Some((y, ys))) => if (self.compare)(x, y) <= 0 {
                    Some((x, self.cursor(xs, ys, None, Some(y))))
                } else {
                    Some((y, self.cursor(xs, ys, Some(x), None)))
                },
            };
        } };
    }
}

public flow merge<T ~ Order>(left: Iter<T>, right: Iter<T>) -> Iter<T> ![] {
    let order = SortedMerge { compare= (a: T, b: T) => a.compare(b) };
    return order.cursor(left, right, None, None);
}

public flow merge_by<T>(left: Iter<T>, right: Iter<T>, compare: (T, T) -> i32 ![]) -> Iter<T> ![] {
    let order = SortedMerge { compare= compare };
    return order.cursor(left, right, None, None);
}

public flow merge<T ~ Order>(left: Array<T>, right: Array<T>) -> Array<T> ![] {
    return merge(left.iter(), right.iter()).collect();
}

public flow merge_by<T>(left: Array<T>, right: Array<T>, compare: (T, T) -> i32 ![]) -> Array<T> ![] {
    return merge_by(left.iter(), right.iter(), compare).collect();
}

public flow binary_search<T ~ Order, S ~ RandomAccess<T>>(values: S, key: T) -> Result<usize, usize> ![] {
    var lower: usize = 0; var upper = values.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = values.get(middle).unwrap().compare(key);
        if order == 0 { return Ok(middle); }
        if order < 0 { lower = middle + 1; } else { upper = middle; }
    }
    return Err(lower);
}
public flow binary_search_by<T, S ~ RandomAccess<T>>(values: S, compare_to_key: (T) -> i32 ![]) -> Result<usize, usize> ![] {
    var lower: usize = 0; var upper = values.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = compare_to_key(values.get(middle).unwrap());
        if order == 0 { return Ok(middle); }
        if order < 0 { lower = middle + 1; } else { upper = middle; }
    }
    return Err(lower);
}
public flow partition<T, effect E>(values: Array<T>, predicate: (T) -> bool ![E]) -> (Array<T>, Array<T>) ![E] {
    var yes: Array<T> = []; var no: Array<T> = [];
    for value in values {
        if predicate(value) { yes = yes.push(value); } else { no = no.push(value); }
    }
    return (yes, no);
}
public flow partition_point<T, S ~ RandomAccess<T>>(values: S, predicate: (T) -> bool ![]) -> usize ![] {
    var lower: usize = 0; var upper = values.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        if predicate(values.get(middle).unwrap()) { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}
public flow is_sorted<T ~ Order>(items: Iter<T>) -> bool ![] {
    var previous: Option<T> = None;
    for value in items {
        match previous { Some(old) => { if old.compare(value) > 0 { return false; } }, None => {}, };
        previous = Some(value);
    }
    return true;
}
public flow is_sorted_by<T>(items: Iter<T>, compare: (T, T) -> i32 ![]) -> bool ![] {
    var previous: Option<T> = None;
    for value in items {
        match previous { Some(old) => { if compare(old, value) > 0 { return false; } }, None => {}, };
        previous = Some(value);
    }
    return true;
}

public flow lower_bound<T ~ Order, S ~ RandomAccess<T>>(values: S, key: T) -> usize ![] {
    var lower: usize = 0; var upper = values.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = values.get(middle).unwrap().compare(key);
        if order < 0 { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}

public flow upper_bound<T ~ Order, S ~ RandomAccess<T>>(values: S, key: T) -> usize ![] {
    var lower: usize = 0; var upper = values.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = values.get(middle).unwrap().compare(key);
        if order <= 0 { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}

public flow lower_bound_by<T, S ~ RandomAccess<T>>(values: S, compare_to_key: (T) -> i32 ![]) -> usize ![] {
    var lower: usize = 0; var upper = values.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = compare_to_key(values.get(middle).unwrap());
        if order < 0 { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}

public flow upper_bound_by<T, S ~ RandomAccess<T>>(values: S, compare_to_key: (T) -> i32 ![]) -> usize ![] {
    var lower: usize = 0; var upper = values.len();
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = compare_to_key(values.get(middle).unwrap());
        if order <= 0 { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}

public flow rotate_left<T>(values: Array<T>, count: usize) -> Array<T> ![] {
    if values.length == 0 { return values; }
    let split = count % values.length;
    var output: Array<T> = [];
    var index = split;
    while index < values.length { output = output.push(values.get(index).unwrap()); index = index + 1; }
    index = 0;
    while index < split { output = output.push(values.get(index).unwrap()); index = index + 1; }
    return output;
}

public flow rotate_right<T>(values: Array<T>, count: usize) -> Array<T> ![] {
    if values.length == 0 { return values; }
    let split = values.length - count % values.length;
    var output: Array<T> = [];
    var index = split;
    while index < values.length { output = output.push(values.get(index).unwrap()); index = index + 1; }
    index = 0;
    while index < split { output = output.push(values.get(index).unwrap()); index = index + 1; }
    return output;
}
