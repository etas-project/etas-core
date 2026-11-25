module std.collections;

type SelectionPriority<K> = { private key: K, private ordinal: usize };
type BoundedSelection<T, K> = { private key: (T) -> K ![], private compare: (K, K) -> i32 ![], private highest: bool };
impl BoundedSelection<T, K> {
    flow select(self, items: Iter<T>, count: usize) -> Array<T> ![] {
        if count == 0 { return []; }
        let compare = self.compare;
        let highest = self.highest;
        var retained: PriorityQueue<T, SelectionPriority<K>> = PriorityQueue.new((a: SelectionPriority<K>, b: SelectionPriority<K>) => {
            let order = compare(a.key, b.key);
            if order != 0 {
                if highest { return if order < 0 { 1 } else { -1 }; }
                return if order > 0 { 1 } else { -1 };
            }
            return if a.ordinal > b.ordinal { 1 } else if a.ordinal < b.ordinal { -1 } else { 0 };
        });
        var ordinal: usize = 0;
        for value in items {
            let selected = (self.key)(value);
            let priority = SelectionPriority { key= selected, ordinal= ordinal };
            if retained.len() < count { retained = retained.push(value, priority); }
            else {
                let (worst, _) = retained.peek_entry().unwrap();
                let order = (self.compare)(selected, worst.key);
                if (self.highest && order > 0) || (!self.highest && order < 0) {
                    let (updated, _) = retained.replace_top(value, priority); retained = updated;
                }
            }
            ordinal = ordinal + 1;
        }
        return retained.to_sorted_array().reverse();
    }
}

type PartitionSelection<T> = { private compare: (T, T) -> i32 ![] };
impl PartitionSelection<T> {
    flow swap(self, values: Array<T>, a: usize, b: usize) -> Array<T> ![] {
        if a == b { return values; }
        let left = values.get(a).unwrap(); let right = values.get(b).unwrap();
        return values.set(a, right)?.unwrap().set(b, left)?.unwrap();
    }
    flow pivot(self, values: Array<T>, start: usize, end: usize) -> T ![] {
        var medians: Array<T> = [];
        var offset = start;
        while offset < end {
            let remaining = end - offset;
            let width = if remaining < 5 { remaining } else { 5 };
            let group = MergeSorter { compare= self.compare }.sorted(Slice { source= values, start= offset, length= width }.to_array());
            medians = medians.push(group.get(width / 2).unwrap());
            offset = offset + width;
        }
        if medians.length == 1 { return medians.get(0).unwrap(); }
        let (_, middle) = self.select(medians, medians.length / 2);
        return middle;
    }
    flow select(self, values: Array<T>, ordinal: usize) -> (Array<T>, T) ![] {
        var output = values;
        var start: usize = 0; var end = values.length;
        while true {
            if end - start == 1 { return (output, output.get(start).unwrap()); }
            let pivot = self.pivot(output, start, end);
            var less = start; var scan = start; var greater = end;
            while scan < greater {
                let order = (self.compare)(output.get(scan).unwrap(), pivot);
                if order < 0 { output = self.swap(output, scan, less); less = less + 1; scan = scan + 1; }
                else if order > 0 { greater = greater - 1; output = self.swap(output, scan, greater); }
                else { scan = scan + 1; }
            }
            if ordinal < less { end = less; }
            else if ordinal >= greater { start = greater; }
            else { return (output, output.get(ordinal).unwrap()); }
        }
    }
}
public flow select_nth<T ~ Order, I ~ Index>(values: Array<T>, index: I) -> (Array<T>, T) ![Error<std.runtime.error.IndexError>] {
    let ordinal = match index.to_usize() { Some(value) => value, None => usize.maximum() };
    if ordinal >= values.length { perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= ordinal, length= values.length }); }
    return PartitionSelection { compare= (a: T, b: T) => a.compare(b) }.select(values, ordinal);
}
public flow select_nth_by<T, I ~ Index>(values: Array<T>, index: I, compare: (T, T) -> i32 ![]) -> (Array<T>, T) ![Error<std.runtime.error.IndexError>] {
    let ordinal = match index.to_usize() { Some(value) => value, None => usize.maximum() };
    if ordinal >= values.length { perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= ordinal, length= values.length }); }
    return PartitionSelection { compare= compare }.select(values, ordinal);
}

public flow top_k<T ~ Order>(items: Iter<T>, k: usize) -> Array<T> ![] {
    let selection = BoundedSelection { key= (value: T) => value, compare= (a: T, b: T) => a.compare(b), highest= true };
    return selection.select(items, k);
}
public flow top_k_by<T>(items: Iter<T>, k: usize, compare: (T, T) -> i32 ![]) -> Array<T> ![] {
    let selection = BoundedSelection { key= (value: T) => value, compare= compare, highest= true };
    return selection.select(items, k);
}
public flow top_k_by_key<T, K ~ Order>(items: Iter<T>, k: usize, key: (T) -> K ![]) -> Array<T> ![] {
    let selection = BoundedSelection { key= key, compare= (a: K, b: K) => a.compare(b), highest= true };
    return selection.select(items, k);
}

public flow bottom_k<T ~ Order>(items: Iter<T>, k: usize) -> Array<T> ![] {
    let selection = BoundedSelection { key= (value: T) => value, compare= (a: T, b: T) => a.compare(b), highest= false };
    return selection.select(items, k);
}
public flow bottom_k_by<T>(items: Iter<T>, k: usize, compare: (T, T) -> i32 ![]) -> Array<T> ![] {
    let selection = BoundedSelection { key= (value: T) => value, compare= compare, highest= false };
    return selection.select(items, k);
}
public flow bottom_k_by_key<T, K ~ Order>(items: Iter<T>, k: usize, key: (T) -> K ![]) -> Array<T> ![] {
    let selection = BoundedSelection { key= key, compare= (a: K, b: K) => a.compare(b), highest= false };
    return selection.select(items, k);
}
