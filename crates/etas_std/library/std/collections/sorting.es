module std.collections;

type MergeSorter<T> = { private compare: (T, T) -> i32 ![] };
impl MergeSorter<T> {
    flow sorted(self, values: Array<T>) -> Array<T> ![] {
        if values.length < 2 { return values; }
        let middle = values.length / 2;
        var left_values: Array<T> = [];
        var right_values: Array<T> = [];
        var index: usize = 0;
        while index < middle {
            left_values = left_values.push(values.get(index).unwrap());
            index = index + 1;
        }
        while index < values.length {
            right_values = right_values.push(values.get(index).unwrap());
            index = index + 1;
        }
        let left = self.sorted(left_values);
        let right = self.sorted(right_values);
        var output: Array<T> = [];
        var a: usize = 0;
        var b: usize = 0;
        while a < left.length && b < right.length {
            let x = left.get(a).unwrap();
            let y = right.get(b).unwrap();
            if (self.compare)(x, y) <= 0 { output = output.push(x); a = a + 1; }
            else { output = output.push(y); b = b + 1; }
        }
        while a < left.length { output = output.push(left.get(a).unwrap()); a = a + 1; }
        while b < right.length { output = output.push(right.get(b).unwrap()); b = b + 1; }
        return output;
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
public flow merge<T ~ Order>(left: Array<T>, right: Array<T>) -> Array<T> ![] {
    var output: Array<T> = [];
    var a: usize = 0;
    var b: usize = 0;
    while a < left.length && b < right.length {
        let x = left.get(a).unwrap(); let y = right.get(b).unwrap();
        if x.compare(y) <= 0 { output = output.push(x); a = a + 1; }
        else { output = output.push(y); b = b + 1; }
    }
    while a < left.length { output = output.push(left.get(a).unwrap()); a = a + 1; }
    while b < right.length { output = output.push(right.get(b).unwrap()); b = b + 1; }
    return output;
}
public flow merge_by<T>(left: Array<T>, right: Array<T>, compare: (T, T) -> i32 ![]) -> Array<T> ![] {
    var output: Array<T> = [];
    var a: usize = 0;
    var b: usize = 0;
    while a < left.length && b < right.length {
        let x = left.get(a).unwrap(); let y = right.get(b).unwrap();
        if compare(x, y) <= 0 { output = output.push(x); a = a + 1; }
        else { output = output.push(y); b = b + 1; }
    }
    while a < left.length { output = output.push(left.get(a).unwrap()); a = a + 1; }
    while b < right.length { output = output.push(right.get(b).unwrap()); b = b + 1; }
    return output;
}
public flow binary_search<T ~ Order>(values: Array<T>, key: T) -> Result<usize, usize> ![] {
    var lower: usize = 0; var upper = values.length;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = values.get(middle).unwrap().compare(key);
        if order == 0 { return Ok(middle); }
        if order < 0 { lower = middle + 1; } else { upper = middle; }
    }
    return Err(lower);
}
public flow binary_search_by<T>(values: Array<T>, compare_to_key: (T) -> i32 ![]) -> Result<usize, usize> ![] {
    var lower: usize = 0; var upper = values.length;
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
public flow partition_point<T>(values: Array<T>, predicate: (T) -> bool ![]) -> usize ![] {
    var lower: usize = 0; var upper = values.length;
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

public flow lower_bound<T ~ Order>(values: Array<T>, key: T) -> usize ![] {
    var lower: usize = 0; var upper = values.length;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = values.get(middle).unwrap().compare(key);
        if order < 0 { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}

public flow upper_bound<T ~ Order>(values: Array<T>, key: T) -> usize ![] {
    var lower: usize = 0; var upper = values.length;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = values.get(middle).unwrap().compare(key);
        if order <= 0 { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}

public flow lower_bound_by<T>(values: Array<T>, compare_to_key: (T) -> i32 ![]) -> usize ![] {
    var lower: usize = 0; var upper = values.length;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let order = compare_to_key(values.get(middle).unwrap());
        if order < 0 { lower = middle + 1; } else { upper = middle; }
    }
    return lower;
}

public flow upper_bound_by<T>(values: Array<T>, compare_to_key: (T) -> i32 ![]) -> usize ![] {
    var lower: usize = 0; var upper = values.length;
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
