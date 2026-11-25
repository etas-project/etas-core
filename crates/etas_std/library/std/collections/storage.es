module std.collections;

// Checked storage adapters expose logical values to the structural runtime
// walker. The algorithms and reconstruction invariants belong to each type.
impl Map<K ~ Hash, V> {
    flow __storage_view(self) -> Array<(K, V)> ![] {
        var entries: Array<(K, V)> = [];
        for slot in self.slots {
            match slot {
                Some(entry) => { entries = entries.push((entry.key, entry.value)); },
                None => {},
            };
        }
        return entries;
    }

    flow __storage_build(entries: Array<(K, V)>) -> Map<K, V> ![] {
        var result: Map<K, V> = Map.new();
        for (key, value) in entries {
            result = result.insert(key, value);
        }
        return result;
    }
}

impl Set<T ~ Hash> {
    flow __storage_view(self) -> Array<T> ![] {
        var values: Array<T> = [];
        for slot in self.values.slots {
            match slot {
                Some(entry) => { values = values.push(entry.key); },
                None => {},
            };
        }
        return values;
    }

    flow __storage_build(values: Array<T>) -> Set<T> ![] {
        var result: Set<T> = Set.new();
        for value in values { result = result.insert(value); }
        return result;
    }
}

impl Slice<T> {
    flow __storage_view(self) -> Array<T> ![] {
        var values: Array<T> = [];
        var offset: usize = 0;
        while offset < self.length {
            values = values.push(self.source.get(self.start + offset).unwrap());
            offset = offset + 1;
        }
        return values;
    }

    flow __storage_build(values: Array<T>) -> Slice<T> ![] {
        return Slice { source= values, start= 0, length= values.length };
    }
}

impl Stack<T> {
    flow __storage_view(self) -> Array<T> ![] {
        var values: Array<T> = [];
        for value in self.items { values = values.push(value); }
        return values;
    }

    flow __storage_build(values: Array<T>) -> Stack<T> ![] {
        var reversed: List<T> = [];
        for value in values { reversed = value :: reversed; }
        return Stack { items= reversed.reverse(), size= values.length };
    }
}

impl Queue<T> {
    flow __storage_view(self) -> Array<T> ![] {
        var values: Array<T> = [];
        for value in self.front { values = values.push(value); }
        for value in self.back.reverse() { values = values.push(value); }
        return values;
    }

    flow __storage_build(values: Array<T>) -> Queue<T> ![] {
        var reversed: List<T> = [];
        for value in values { reversed = value :: reversed; }
        return Queue { front= reversed.reverse(), back= [], size= values.length };
    }
}

impl Deque<T> {
    flow __storage_view(self) -> Array<T> ![] {
        var values: Array<T> = [];
        for value in self.front { values = values.push(value); }
        for value in self.back.reverse() { values = values.push(value); }
        return values;
    }

    flow __storage_build(values: Array<T>) -> Deque<T> ![] {
        var reversed: List<T> = [];
        for value in values { reversed = value :: reversed; }
        return Deque { front= reversed.reverse(), back= [], size= values.length }.balanced();
    }
}

impl BoundedDeque<T> {
    flow __storage_view(self) -> { values: Array<T>, maximum: usize, overflow: OverflowPolicy } ![] {
        var values: Array<T> = [];
        for value in self.values.front { values = values.push(value); }
        for value in self.values.back.reverse() { values = values.push(value); }
        return { values= values, maximum= self.maximum, overflow= self.overflow };
    }

    flow __storage_build(view: { values: Array<T>, maximum: usize, overflow: OverflowPolicy }) -> Option<BoundedDeque<T>> ![] {
        if view.maximum == 0 || view.values.length > view.maximum { return None; }
        var reversed: List<T> = [];
        for value in view.values { reversed = value :: reversed; }
        let values = Deque { front= reversed.reverse(), back= [], size= view.values.length }.balanced();
        return Some(BoundedDeque { values= values, maximum= view.maximum, overflow= view.overflow });
    }
}
