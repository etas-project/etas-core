module app.sequence_algorithms;

import std.collections.{Iterable, Iter, RandomAccess, binary_search, binary_search_by,
    lower_bound, upper_bound, lower_bound_by, upper_bound_by, partition_point, merge, merge_by,
    starts_with, find_sequence, find_all_sequences, split_on, split_when, split_sequence,
    split_when_collect};

// An indexed sequence with no backing array. A search must only probe the requested ordinals.
type Evens = { count: usize };
impl Evens ~ Iterable<usize> {
    public flow iter(self) -> Iter<usize> ![] {
        return Iter.init(self.count, (i: usize) => i * 2);
    }
}
impl Evens ~ RandomAccess<usize> {
    public flow len(self) -> usize ![] { return self.count; }
    public flow get<I ~ Index>(self, index: I) -> Option<usize> ![] {
        let i = match index.to_usize() { Some(value) => value, None => { return None; } };
        return if i < self.count { Some(i * 2) } else { None };
    }
}

type Ordinal = { value: usize };
impl Ordinal ~ Index {
    public flow to_usize(self) -> Option<usize> ![] { return Some(self.value); }
}

public flow main(args: Array<string>) -> i32 {
    let virtual = Evens { count= 1000000 };
    let key: usize = 123456;
    std.core.assert(binary_search(virtual, key) == Ok(61728));
    std.core.assert(partition_point(virtual, (v: usize) => v < key) == 61728);
    let data: Array<i32> = [0, 2, 2, 2, 4];
    let view = data[1, 5)?.unwrap();
    std.core.assert(lower_bound(view, 2) == 0 && upper_bound(view, 2) == 3);
    std.core.assert(binary_search_by(view, (v: i32) => v.compare(3)) == Err(3));
    let raw = "aab".utf8;
    let third = Ordinal { value= 2 };
    std.core.assert(raw.get(third) == Some(98) && data.get(third) == Some(2));
    std.core.assert(lower_bound_by(raw, (v: u8) => v.compare(98)) == 2);
    std.core.assert(upper_bound_by(raw, (v: u8) => v.compare(98)) == 3);

    let left: Array<(i32, i32)> = [(1, 1), (2, 2)];
    let right: Array<(i32, i32)> = [(1, 3), (2, 4)];
    let by_key = (a: (i32, i32), b: (i32, i32)) => { let (x, _) = a; let (y, _) = b; return x.compare(y); };
    let merged = merge_by(left.iter(), right.iter(), by_key);
    std.core.assert(merged.take(2).collect() == [(1, 1), (1, 3)]);
    std.core.assert(merged.collect() == merge_by(left, right, by_key));

    // A consumer of the first match must never request the remaining input.
    let prefix = Iter.init(1000000, (i: usize) => {
        std.core.assert(i < 2);
        return i;
    });
    std.core.assert(find_sequence(prefix, [0, 1]) == Some(0));
    std.core.assert(merge(prefix, Iter.once(8)).take(1).collect() == [0]);

    let values: List<i32> = 1 :: 1 :: 1 :: 1 :: [];
    std.core.assert(starts_with(values.iter(), [1, 1]));
    std.core.assert(find_sequence(values.iter(), [1, 1, 1]) == Some(0));
    std.core.assert(find_all_sequences(values.iter(), [1, 1, 1], true).collect() == [(0, 3), (1, 4)]);
    std.core.assert(find_all_sequences(values.iter(), [1, 1]).collect() == [(0, 2), (2, 4)]);
    std.core.assert(find_all_sequences(data.iter().take(1), []).collect() == [(0, 0), (1, 1)]);
    let runs: Array<i32> = [2, 1, 2, 1, 2, 1];
    std.core.assert(split_sequence(runs.iter(), [2, 1]).collect() == [[], [], [], []]);
    std.core.assert(split_sequence(runs.iter().take(0), []).collect() == [[], []]);
    std.core.assert(split_sequence(runs.iter().take(2), []).collect() == [[], [2], [1], []]);
    let predicate = (v: i32) => v == 2;
    let parts = split_when_collect(runs.iter(), predicate);
    std.core.assert(parts == [[], [1], [1], [1]]);
    std.core.assert(split_on(runs.iter(), 2).collect() == parts);
    std.core.assert(split_when(runs.iter(), predicate).collect() == parts);
    return 0;
}
