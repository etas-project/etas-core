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
        // Host maps arrive in insertion order, before Etas has run the key's Hash implementation.
        if self.buckets.is_empty() {
            let count = self.slots.length;
            if count == 0 { return None; }
            let index = self.read_index();
            var slot = index.get(usize { bits= hash.bits % count.bits }).unwrap();
            while slot < count {
                let entry = self.slots.get(slot).unwrap().unwrap();
                if entry.key == key { return Some(slot); }
                slot = index.get(count + slot).unwrap();
            }
            return None;
        }
        let bucket = usize { bits= hash.bits % self.buckets.length.bits };
        for slot in self.buckets.get(bucket).unwrap() {
            let entry = self.slots.get(slot).unwrap().unwrap();
            if entry.hash == hash && entry.key == key { return Some(slot); }
        }
        return None;
    }
    // Import leaves insertion slots intact. Build bucket heads and next links
    // once with K's checked Hash and the original seed, even on a read-only map.
    flow read_index(self) -> Array<usize> ![] {
        let seed = self.base.finish();
        let cached = ArrayStorage.cached_map_index(self.slots, seed);
        if cached.length != 0 { return cached; }
        let count = self.slots.length;
        var index = Array.repeat(count + count, count);
        var slot: usize = 0;
        for candidate in self.slots {
            match candidate {
                Some(entry) => {
                    let hash = entry.key.hash(self.base).finish();
                    let bucket = usize { bits= hash.bits % count.bits };
                    let head = index.get(bucket).unwrap();
                    index = index.set(count + slot, head)?.unwrap();
                    index = index.set(bucket, slot)?.unwrap();
                },
                None => {},
            };
            slot = slot + 1;
        }
        ArrayStorage.remember_map_index(self.slots, seed, index);
        return index;
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
                    let indexed = if self.buckets.is_empty() { entry { hash= entry.key.hash(self.base).finish() } } else { entry };
                    let bucket = usize { bits= indexed.hash.bits % capacity.bits };
                    let chain = slot :: (buckets.get(bucket).unwrap());
                    buckets = buckets.set(bucket, chain)?.unwrap();
                    slots = slots.push(Some(indexed));
                },
            };
        }
        return Map { buckets= buckets, slots= slots, length= self.length, base= self.base };
    }
    public flow insert(self, key: K, value: V) -> Map<K, V> ![] {
        let indexed = if self.buckets.is_empty() { self.reindexed(8 + self.length * 2) } else { self };
        let hash = key.hash(indexed.base).finish();
        match indexed.locate(key, hash) {
            Some(slot) => {
                let old = indexed.slots.get(slot).unwrap().unwrap();
                let entry = MapEntry { key= old.key, value= value, hash= old.hash };
                return Map { buckets= indexed.buckets, slots= indexed.slots.set(slot, Some(entry))?.unwrap(), length= indexed.length, base= indexed.base };
            },
            None => {},
        };
        return indexed.appended(key, value, hash);
    }
    flow appended(self, key: K, value: V, hash: u64) -> Map<K, V> ![] {
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
        let indexed = if self.buckets.is_empty() { self.reindexed(8 + self.length * 2) } else { self };
        let hash = key.hash(indexed.base).finish();
        let slot = match indexed.locate(key, hash) { None => { return (indexed, None); }, Some(slot) => slot };
        let old = indexed.slots.get(slot).unwrap().unwrap();
        return (indexed.without_slot(slot, hash), Some(old.value));
    }
    flow without_slot(self, slot: usize, hash: u64) -> Map<K, V> ![] {
        let length = self.length - 1;
        if length == 0 { return Map { buckets= Array.repeat(8, []), slots= [], length= 0, base= self.base }; }
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
            return updated.reindexed(updated.buckets.length / 2);
        }
        if updated.slots.length - length > length + 8 { return updated.reindexed(updated.buckets.length); }
        return updated;
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
        let indexed = if self.buckets.is_empty() { self.reindexed(8 + self.length * 2) } else { self };
        let hash = key.hash(indexed.base).finish();
        let slot = indexed.locate(key, hash);
        let old = match slot {
            Some(at) => Some(indexed.slots.get(at).unwrap().unwrap().value),
            None => None,
        };
        return match f(old) {
            Some(value) => match slot {
                Some(at) => {
                    let entry = indexed.slots.get(at).unwrap().unwrap();
                    indexed { slots= indexed.slots.set(at, Some(entry { value= value }))?.unwrap() }
                },
                None => indexed.appended(key, value, hash),
            },
            None => match slot {
                Some(at) => indexed.without_slot(at, hash),
                None => indexed,
            },
        };
    }
    public flow merge<effect E>(self, other: Map<K, V>, resolve: (K, V, V) -> V ![E]) -> Map<K, V> ![E] {
        var output = if self.buckets.is_empty() { self.reindexed(8 + self.length * 2) } else { self };
        for (key, right) in other {
            let hash = key.hash(output.base).finish();
            match output.locate(key, hash) {
                Some(slot) => {
                    let old = output.slots.get(slot).unwrap().unwrap();
                    let value = resolve(key, old.value, right);
                    output = output { slots= output.slots.set(slot, Some(old { value= value }))?.unwrap() };
                },
                None => { output = output.appended(key, right, hash); },
            };
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
    public flow insert(self, value: T) -> Set<T> ![] {
        let values = if self.values.buckets.is_empty() { self.values.reindexed(8 + self.values.length * 2) } else { self.values };
        let hash = value.hash(values.base).finish();
        if values.locate(value, hash).is_some() { return Set { values= values }; }
        return Set { values= values.appended(value, (), hash) };
    }
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
