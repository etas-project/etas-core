module std.collections;

public flow group_by<T, K ~ Hash, effect E>(items: Iter<T>, key: (T) -> K ![E]) -> Map<K, Array<T>> ![E] {
    var groups: Map<K, Array<T>> = Map.new();
    for value in items {
        let name = key(value);
        let group = match groups.get(name) { None => [], Some(group) => group };
        groups = groups.insert(name, group.push(value));
    }
    return groups;
}
public flow count_by<T, K ~ Hash, effect E>(items: Iter<T>, key: (T) -> K ![E]) -> Map<K, usize> ![E] {
    var counts: Map<K, usize> = Map.new();
    for value in items {
        let name = key(value);
        let count = counts.get(name).unwrap_or(0);
        counts = counts.insert(name, count + 1);
    }
    return counts;
}
public flow frequencies<T ~ Hash>(items: Iter<T>) -> Map<T, usize> ![] {
    var counts: Map<T, usize> = Map.new();
    for value in items {
        let count = counts.get(value).unwrap_or(0);
        counts = counts.insert(value, count + 1);
    }
    return counts;
}
public flow merge_counts<K ~ Hash>(left: Map<K, usize>, right: Map<K, usize>) -> Map<K, usize> ![] {
    var output = left;
    for (key, count) in right {
        let previous = output.get(key).unwrap_or(0);
        output = output.insert(key, previous + count);
    }
    return output;
}

type AdjacentGroups<T, K ~ Eq> = {
    private items: Iter<T>,
    private pending: Option<(K, T)>,
    private key: (T) -> K ![],
};
impl AdjacentGroups<T, K ~ Eq> {
    flow iter(self) -> Iter<(K, Array<T>)> ![] {
        return Iter { next= () => {
            let (name, first, items) = match self.pending {
                Some((name, value)) => (name, value, self.items),
                None => match (self.items.next)() {
                    None => { return None; },
                    Some((value, tail)) => ((self.key)(value), value, tail),
                },
            };
            var group: Array<T> = [first];
            var cursor = items;
            while true {
                match (cursor.next)() {
                    None => { return Some(((name, group), Iter.empty())); },
                    Some((value, tail)) => {
                        let next_name = (self.key)(value);
                        if next_name != name {
                            let remaining = AdjacentGroups { items= tail, pending= Some((next_name, value)), key= self.key };
                            return Some(((name, group), remaining.iter()));
                        }
                        group = group.push(value);
                        cursor = tail;
                    },
                };
            }
        } };
    }
}
public flow group_adjacent_by<T, K ~ Eq>(items: Iter<T>, key: (T) -> K ![]) -> Iter<(K, Array<T>)> ![] {
    return AdjacentGroups { items= items, pending= None, key= key }.iter();
}

public flow dedup<T ~ Eq>(values: Array<T>) -> Array<T> ![] {
    var output: Array<T> = [];
    var previous: Option<T> = None;
    for value in values {
        let duplicate = match previous { None => false, Some(old) => old == value };
        if !duplicate { output = output.push(value); previous = Some(value); }
    }
    return output;
}
public flow dedup_by<T>(values: Array<T>, equal: (T, T) -> bool ![]) -> Array<T> ![] {
    var output: Array<T> = [];
    var previous: Option<T> = None;
    for value in values {
        let duplicate = match previous { None => false, Some(old) => equal(old, value) };
        if !duplicate { output = output.push(value); previous = Some(value); }
    }
    return output;
}
public flow dedup_by_key<T, K ~ Eq>(values: Array<T>, key: (T) -> K ![]) -> Array<T> ![] {
    var output: Array<T> = [];
    var previous: Option<K> = None;
    for value in values {
        let current = key(value);
        let duplicate = match previous { None => false, Some(old) => old == current };
        if !duplicate { output = output.push(value); previous = Some(current); }
    }
    return output;
}
public flow unique<T ~ Hash>(values: Array<T>) -> Array<T> ![] {
    var output: Array<T> = [];
    var seen: Set<T> = Set.new();
    for value in values {
        if !seen.contains(value) { seen = seen.insert(value); output = output.push(value); }
    }
    return output;
}
public flow unique_by_key<T, K ~ Hash>(values: Array<T>, key: (T) -> K ![]) -> Array<T> ![] {
    var output: Array<T> = [];
    var seen: Set<K> = Set.new();
    for value in values {
        let current = key(value);
        if !seen.contains(current) { seen = seen.insert(current); output = output.push(value); }
    }
    return output;
}
