module std.collections;

impl Map<K ~ Hash, V> {
    public flow new() -> Map<K, V> ![] {
        return Map { buckets= Array.repeat(8, []), slots= [], length= 0, base= Hasher.new() };
    }
    public flow with_hasher(base: Hasher) -> Map<K, V> ![] {
        return Map { buckets= Array.repeat(8, []), slots= [], length= 0, base= base };
    }
    public flow from_entries(items: Iter<(K, V)>) -> Map<K, V> ![] {
        var output: Map<K, V> = Map.new();
        for (key, value) in items { output = output.insert(key, value); }
        return output;
    }
    public flow from_entries(items: List<(K, V)>) -> Map<K, V> ![] {
        var output: Map<K, V> = Map.new();
        for (key, value) in items { output = output.insert(key, value); }
        return output;
    }
    public flow len(self) -> usize ![] { return self.length; }
    public flow is_empty(self) -> bool ![] { return self.length == 0; }
    flow locate(self, key: K, hash: u64) -> Option<usize> ![] {
        let bucket = usize { bits= hash.bits % self.buckets.length.bits };
        for slot in self.buckets.get(bucket).unwrap() {
            let entry = self.slots.get(slot).unwrap().unwrap();
            if entry.hash == hash && entry.key == key { return Some(slot); }
        }
        return None;
    }
    public flow get(self, key: K) -> Option<V> ![] {
        let hash = key.hash(self.base).finish();
        return match self.locate(key, hash) {
            None => None,
            Some(slot) => Some(self.slots.get(slot).unwrap().unwrap().value),
        };
    }
    public flow contains_key(self, key: K) -> bool ![] {
        let hash = key.hash(self.base).finish();
        return self.locate(key, hash).is_some();
    }
    flow reindexed(self, capacity: usize) -> Map<K, V> ![] {
        var buckets: Array<List<usize>> = Array.repeat(capacity, []);
        var slots: Array<Option<MapEntry<K, V>>> = [];
        for candidate in self.slots {
            match candidate {
                None => {},
                Some(entry) => {
                    let slot = slots.length;
                    let bucket = usize { bits= entry.hash.bits % capacity.bits };
                    let chain = slot :: (buckets.get(bucket).unwrap());
                    buckets = buckets.set(bucket, chain)?.unwrap();
                    slots = slots.push(Some(entry));
                },
            };
        }
        return Map { buckets= buckets, slots= slots, length= self.length, base= self.base };
    }
    public flow insert(self, key: K, value: V) -> Map<K, V> ![] {
        let hash = key.hash(self.base).finish();
        match self.locate(key, hash) {
            Some(slot) => {
                let old = self.slots.get(slot).unwrap().unwrap();
                let entry = MapEntry { key= old.key, value= value, hash= old.hash };
                return Map { buckets= self.buckets, slots= self.slots.set(slot, Some(entry))?.unwrap(), length= self.length, base= self.base };
            },
            None => {},
        };
        let grown = if self.length + 1 > self.buckets.length - self.buckets.length / 4 {
            self.reindexed(self.buckets.length * 2)
        } else { self };
        let bucket = usize { bits= hash.bits % grown.buckets.length.bits };
        let chain = grown.slots.length :: (grown.buckets.get(bucket).unwrap());
        let buckets = grown.buckets.set(bucket, chain)?.unwrap();
        let slots = grown.slots.push(Some(MapEntry { key= key, value= value, hash= hash }));
        return Map { buckets= buckets, slots= slots, length= grown.length + 1, base= grown.base };
    }
    flow removed(self, key: K) -> (Map<K, V>, Option<V>) ![] {
        let hash = key.hash(self.base).finish();
        let slot = match self.locate(key, hash) { None => { return (self, None); }, Some(slot) => slot };
        let old = self.slots.get(slot).unwrap().unwrap();
        let length = self.length - 1;
        if length == 0 { return (Map { buckets= Array.repeat(8, []), slots= [], length= 0, base= self.base }, Some(old.value)); }
        let bucket = usize { bits= hash.bits % self.buckets.length.bits };
        var chain: List<usize> = [];
        for candidate in self.buckets.get(bucket).unwrap() {
            if candidate != slot { chain = candidate :: chain; }
        }
        let updated = Map {
            buckets= self.buckets.set(bucket, chain)?.unwrap(),
            slots= self.slots.set(slot, None)?.unwrap(),
            length= length, base= self.base,
        };
        if length < updated.buckets.length / 4 && updated.buckets.length > 8 {
            return (updated.reindexed(updated.buckets.length / 2), Some(old.value));
        }
        if updated.slots.length - length > length + 8 {
            return (updated.reindexed(updated.buckets.length), Some(old.value));
        }
        return (updated, Some(old.value));
    }
    public flow remove(self, key: K) -> Map<K, V> ![] {
        let (updated, _) = self.removed(key);
        return updated;
    }
    public flow take(self, key: K) -> (Map<K, V>, Option<V>) ![] {
        let (updated, old) = self.removed(key);
        return (updated, old);
    }
    public flow update<effect E>(self, key: K, f: (Option<V>) -> Option<V> ![E]) -> Map<K, V> ![E] {
        return match f(self.get(key)) { Some(value) => self.insert(key, value), None => self.remove(key) };
    }
    public flow merge<effect E>(self, other: Map<K, V>, resolve: (K, V, V) -> V ![E]) -> Map<K, V> ![E] {
        var output = self;
        for (key, right) in other {
            let value = match output.get(key) { None => right, Some(left) => resolve(key, left, right) };
            output = output.insert(key, value);
        }
        return output;
    }
    public flow keys(self) -> Iter<K> ![] {
        return self.iter().map((entry: (K, V)) => { let (key, _) = entry; return key; });
    }
    public flow values(self) -> Iter<V> ![] {
        return self.iter().map((entry: (K, V)) => { let (_, value) = entry; return value; });
    }
    public flow clear(self) -> Map<K, V> ![] {
        return Map { buckets= Array.repeat(8, []), slots= [], length= 0, base= self.base };
    }
    flow cursor(self, offset: usize) -> Iter<(K, V)> ![] {
        return Iter { next= () => {
            var slot = offset;
            while slot < self.slots.length {
                let next_slot = slot + 1;
                match self.slots.get(slot).unwrap() {
                    None => { slot = next_slot; },
                    Some(entry) => { return Some(((entry.key, entry.value), self.cursor(next_slot))); },
                };
            }
            return None;
        } };
    }
}
impl Map<K ~ Hash, V> ~ Iterable<(K, V)> {
    public flow iter(self) -> Iter<(K, V)> ![] { return self.cursor(0); }
}

impl Set<T ~ Hash> {
    public flow new() -> Set<T> ![] { return Set { values= Map.new() }; }
    public flow with_hasher(base: Hasher) -> Set<T> ![] { return Set { values= Map.with_hasher(base) }; }
    public flow from_iter(items: Iter<T>) -> Set<T> ![] {
        var values: Map<T, unit> = Map.new();
        for value in items { values = values.insert(value, ()); }
        return Set { values= values };
    }
    public flow from_list(items: List<T>) -> Set<T> ![] {
        var values: Map<T, unit> = Map.new();
        for value in items { values = values.insert(value, ()); }
        return Set { values= values };
    }
    public flow len(self) -> usize ![] { return self.values.len(); }
    public flow is_empty(self) -> bool ![] { return self.values.is_empty(); }
    public flow contains(self, value: T) -> bool ![] { return self.values.contains_key(value); }
    public flow insert(self, value: T) -> Set<T> ![] { return Set { values= self.values.insert(value, ()) }; }
    public flow remove(self, value: T) -> Set<T> ![] { return Set { values= self.values.remove(value) }; }
    public flow union(self, other: Set<T>) -> Set<T> ![] {
        var values = self.values;
        for value in other { values = values.insert(value, ()); }
        return Set { values= values };
    }
    public flow intersection(self, other: Set<T>) -> Set<T> ![] {
        var values: Map<T, unit> = Map.with_hasher(self.values.base);
        for value in self { if other.contains(value) { values = values.insert(value, ()); } }
        return Set { values= values };
    }
    public flow difference(self, other: Set<T>) -> Set<T> ![] {
        var values: Map<T, unit> = Map.with_hasher(self.values.base);
        for value in self { if !other.contains(value) { values = values.insert(value, ()); } }
        return Set { values= values };
    }
    public flow symmetric_difference(self, other: Set<T>) -> Set<T> ![] {
        var values: Map<T, unit> = Map.with_hasher(self.values.base);
        for value in self { if !other.contains(value) { values = values.insert(value, ()); } }
        for value in other { if !self.contains(value) { values = values.insert(value, ()); } }
        return Set { values= values };
    }
    public flow is_subset(self, other: Set<T>) -> bool ![] {
        for value in self { if !other.contains(value) { return false; } }
        return true;
    }
    public flow is_disjoint(self, other: Set<T>) -> bool ![] {
        for value in self { if other.contains(value) { return false; } }
        return true;
    }
    public flow clear(self) -> Set<T> ![] { return Set { values= self.values.clear() }; }
}
impl Set<T ~ Hash> ~ Iterable<T> {
    public flow iter(self) -> Iter<T> ![] { return self.values.keys(); }
}
