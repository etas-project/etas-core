module std.collections;

type PriorityEntry<T, P> = { value: T, priority: P, order: usize };

impl PriorityEntry<T, P> {
    flow before(self, other: PriorityEntry<T, P>, compare: (P, P) -> i32 ![]) -> bool ![] {
        let order = compare(self.priority, other.priority);
        return order > 0 || (order == 0 && self.order < other.order);
    }
}

public type PriorityQueue<T, P> = {
    private slots: Array<Option<PriorityEntry<T, P>>>,
    private heap: Array<usize>,
    private size: usize,
    private compare: (P, P) -> i32 ![],
};

impl PriorityQueue<T, P ~ Order> {
    public flow new() -> PriorityQueue<T, P> ![] {
        return PriorityQueue {
            slots= [],
            heap= [],
            size= 0,
            compare= (left: P, right: P) => left.compare(right),
        };
    }
}

impl PriorityQueue<T, P> {
    public flow new(compare: (P, P) -> i32 ![]) -> PriorityQueue<T, P> ![] {
        return PriorityQueue {
            slots= [],
            heap= [],
            size= 0,
            compare= compare,
        };
    }

    public flow push(self, value: T, priority: P) -> PriorityQueue<T, P> ![] {
        let slot = self.slots.length;
        let slots = self.slots.push(Some(PriorityEntry { value= value, priority= priority, order= slot }));
        var heap = self.heap;
        if self.size == heap.length {
            heap = heap.push(slot);
        } else {
            heap = heap.set(self.size, slot)?.unwrap();
        }
        var child = self.size;
        while child > 0 {
            let parent = (child - 1) / 2;
            let child_slot = heap.get(child).unwrap();
            let parent_slot = heap.get(parent).unwrap();
            let child_entry = slots.get(child_slot).unwrap().unwrap();
            let parent_entry = slots.get(parent_slot).unwrap().unwrap();
            if !child_entry.before(parent_entry, self.compare) {
                break;
            }
            heap = heap.set(child, parent_slot)?.unwrap();
            heap = heap.set(parent, child_slot)?.unwrap();
            child = parent;
        }
        return PriorityQueue {
            slots= slots,
            heap= heap,
            size= self.size + 1,
            compare= self.compare,
        };
    }

    public flow pop(self) -> (PriorityQueue<T, P>, Option<T>) ![] {
        if self.size == 0 {
            return (self, None);
        }
        let removed_slot = self.heap.get(0).unwrap();
        let removed = self.slots.get(removed_slot).unwrap().unwrap();
        let size = self.size - 1;
        let slots = self.slots.set(removed_slot, None)?.unwrap();
        var heap = self.heap;
        if size > 0 {
            heap = heap.set(0, heap.get(size).unwrap())?.unwrap();
            var parent: usize = 0;
            while parent < size / 2 {
                let left = parent * 2 + 1;
                let right = left + 1;
                var child = left;
                if right < size {
                    let right_entry = slots.get(heap.get(right).unwrap()).unwrap().unwrap();
                    let left_entry = slots.get(heap.get(left).unwrap()).unwrap().unwrap();
                    if right_entry.before(left_entry, self.compare) {
                        child = right;
                    }
                }
                let child_slot = heap.get(child).unwrap();
                let parent_slot = heap.get(parent).unwrap();
                let child_entry = slots.get(child_slot).unwrap().unwrap();
                let parent_entry = slots.get(parent_slot).unwrap().unwrap();
                if !child_entry.before(parent_entry, self.compare) {
                    break;
                }
                heap = heap.set(parent, child_slot)?.unwrap();
                heap = heap.set(child, parent_slot)?.unwrap();
                parent = child;
            }
        }
        if size == 0 {
            return (PriorityQueue { slots= [], heap= [], size= 0, compare= self.compare }, Some(removed.value));
        }
        let updated = PriorityQueue { slots= slots, heap= heap, size= size, compare= self.compare };
        if slots.length - size > size + 8 { return (updated.compacted(), Some(removed.value)); }
        return (updated, Some(removed.value));
    }

    public flow len(self) -> usize ![] {
        return self.size;
    }

    public flow is_empty(self) -> bool ![] {
        return self.size == 0;
    }
}

impl PriorityQueue<T, P> ~ Iterable<(P, T)> {
    public flow iter(self) -> Iter<(P, T)> ![] {
        return Iter { next= () => {
            var slot: usize = 0;
            while slot < self.slots.length {
                match self.slots.get(slot).unwrap() {
                    Some(entry) => {
                        return Some(((entry.priority, entry.value), self.iter_from(slot + 1)));
                    },
                    None => { slot = slot + 1; },
                };
            }
            return None;
        } };
    }
}

impl PriorityQueue<T, P> {
    flow iter_from(self, offset: usize) -> Iter<(P, T)> ![] {
        return Iter { next= () => {
            var slot = offset;
            while slot < self.slots.length {
                match self.slots.get(slot).unwrap() {
                    Some(entry) => {
                        return Some(((entry.priority, entry.value), self.iter_from(slot + 1)));
                    },
                    None => { slot = slot + 1; },
                };
            }
            return None;
        } };
    }
}

