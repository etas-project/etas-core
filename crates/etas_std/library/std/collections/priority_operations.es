module std.collections;

impl PriorityQueue<T, P> {
    flow compacted(self) -> PriorityQueue<T, P> ![] {
        var slots: Array<Option<PriorityEntry<T, P>>> = [];
        var remap: Array<usize> = Array.repeat(self.slots.length, 0);
        var old_slot: usize = 0;
        for candidate in self.slots {
            match candidate {
                None => {},
                Some(entry) => {
                    let slot = slots.length;
                    remap = remap.set(old_slot, slot)?.unwrap();
                    slots = slots.push(Some(PriorityEntry { value= entry.value, priority= entry.priority, order= slot }));
                },
            };
            old_slot = old_slot + 1;
        }
        var heap: Array<usize> = [];
        var index: usize = 0;
        while index < self.size {
            heap = heap.push(remap.get(self.heap.get(index).unwrap()).unwrap());
            index = index + 1;
        }
        return PriorityQueue { slots= slots, heap= heap, size= self.size, compare= self.compare };
    }
    flow heapified(self) -> PriorityQueue<T, P> ![] {
        var heap = self.heap;
        var root = self.size / 2;
        while root > 0 {
            root = root - 1;
            var parent = root;
            while parent < self.size / 2 {
                let left = parent * 2 + 1;
                let right = left + 1;
                var child = left;
                if right < self.size && self.slots.get(heap.get(right).unwrap()).unwrap().unwrap().before(self.slots.get(heap.get(left).unwrap()).unwrap().unwrap(), self.compare) { child = right; }
                let child_slot = heap.get(child).unwrap();
                let parent_slot = heap.get(parent).unwrap();
                if !self.slots.get(child_slot).unwrap().unwrap().before(self.slots.get(parent_slot).unwrap().unwrap(), self.compare) { break; }
                heap = heap.set(parent, child_slot)?.unwrap();
                heap = heap.set(child, parent_slot)?.unwrap();
                parent = child;
            }
        }
        return PriorityQueue { slots= self.slots, heap= heap, size= self.size, compare= self.compare };
    }
    public flow from_entries(items: Iter<(P, T)>, compare: (P, P) -> i32 ![]) -> PriorityQueue<T, P> ![] {
        var slots: Array<Option<PriorityEntry<T, P>>> = [];
        var heap: Array<usize> = [];
        for (priority, value) in items {
            let index = slots.length;
            slots = slots.push(Some(PriorityEntry { value= value, priority= priority, order= index }));
            heap = heap.push(index);
        }
        let queue = PriorityQueue { slots= slots, heap= heap, size= heap.length, compare= compare };
        return queue.heapified();
    }
    public flow peek(self) -> Option<T> ![] {
        if self.size == 0 { return None; }
        return Some(self.slots.get(self.heap.get(0).unwrap()).unwrap().unwrap().value);
    }
    public flow peek_entry(self) -> Option<(P, T)> ![] {
        if self.size == 0 { return None; }
        let entry = self.slots.get(self.heap.get(0).unwrap()).unwrap().unwrap();
        return Some((entry.priority, entry.value));
    }
    public flow pop_entry(self) -> (PriorityQueue<T, P>, Option<(P, T)>) ![] {
        let entry = self.peek_entry();
        let (remaining, _) = self.pop();
        return (remaining, entry);
    }
    public flow push_pop(self, value: T, priority: P) -> (PriorityQueue<T, P>, T) ![] {
        if self.size == 0 { return (self, value); }
        let top = self.slots.get(self.heap.get(0).unwrap()).unwrap().unwrap();
        if (self.compare)(priority, top.priority) > 0 { return (self, value); }
        let (updated, removed) = self.replace_top(value, priority);
        return (updated, removed.unwrap());
    }
    public flow replace_top(self, value: T, priority: P) -> (PriorityQueue<T, P>, Option<T>) ![] {
        if self.size == 0 { return (self.push(value, priority), None); }
        let old_slot = self.heap.get(0).unwrap();
        let previous = self.slots.get(old_slot).unwrap().unwrap();
        let slot = self.slots.length;
        let entry = PriorityEntry { value= value, priority= priority, order= slot };
        let slots = self.slots.set(old_slot, None)?.unwrap().push(Some(entry));
        var heap = self.heap;
        var parent: usize = 0;
        while parent < self.size / 2 {
            let left = parent * 2 + 1;
            let right = left + 1;
            var child = left;
            if right < self.size && slots.get(heap.get(right).unwrap()).unwrap().unwrap().before(slots.get(heap.get(left).unwrap()).unwrap().unwrap(), self.compare) {
                child = right;
            }
            let child_slot = heap.get(child).unwrap();
            if !slots.get(child_slot).unwrap().unwrap().before(entry, self.compare) { break; }
            heap = heap.set(parent, child_slot)?.unwrap();
            parent = child;
        }
        heap = heap.set(parent, slot)?.unwrap();
        let updated = PriorityQueue { slots= slots, heap= heap, size= self.size, compare= self.compare };
        if slots.length - self.size > self.size + 8 { return (updated.compacted(), Some(previous.value)); }
        return (updated, Some(previous.value));
    }
    public flow to_sorted_array(self) -> Array<T> ![] {
        var queue = self;
        var output: Array<T> = [];
        while !queue.is_empty() {
            let (remaining, value) = queue.pop();
            output = output.push(value.unwrap()); queue = remaining;
        }
        return output;
    }
    public flow clear(self) -> PriorityQueue<T, P> ![] {
        return PriorityQueue { slots= [], heap= [], size= 0, compare= self.compare };
    }
}
impl PriorityQueue<T, P ~ Order> {
    public flow from_entries(items: Iter<(P, T)>) -> PriorityQueue<T, P> ![] {
        var slots: Array<Option<PriorityEntry<T, P>>> = [];
        var heap: Array<usize> = [];
        for (priority, value) in items {
            let index = slots.length;
            slots = slots.push(Some(PriorityEntry { value= value, priority= priority, order= index }));
            heap = heap.push(index);
        }
        let queue = PriorityQueue { slots= slots, heap= heap, size= heap.length, compare= (a: P, b: P) => a.compare(b) };
        return queue.heapified();
    }
}
