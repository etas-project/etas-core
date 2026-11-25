module std.collections;

public type Stack<T> = { private items: List<T>, private size: usize };

impl Stack<T> {
    public flow new() -> Stack<T> ![] {
        return Stack { items= [], size= 0 };
    }

    public flow push(self, value: T) -> Stack<T> ![] {
        return Stack { items= value :: self.items, size= self.size + 1 };
    }

    public flow pop(self) -> (Stack<T>, Option<T>) ![] {
        return match self.items {
            [] => (self, None),
            head :: tail => (Stack { items= tail, size= self.size - 1 }, Some(head)),
        };
    }

    public flow len(self) -> usize ![] {
        return self.size;
    }

    public flow is_empty(self) -> bool ![] {
        return self.size == 0;
    }
}


public type Queue<T> = { private front: List<T>, private back: List<T>, private size: usize };

impl Queue<T> {
    public flow new() -> Queue<T> ![] {
        return Queue { front= [], back= [], size= 0 };
    }
    public flow push(self, value: T) -> Queue<T> ![] {
        if self.size == 0 { return Queue { front= value :: [], back= [], size= 1 }; }
        return Queue { front= self.front, back= value :: self.back, size= self.size + 1 };
    }
    public flow pop(self) -> (Queue<T>, Option<T>) ![] {
        return match self.front {
            [] => (self, None),
            head :: tail => {
                let front = if tail.is_empty() { self.back.reverse() } else { tail };
                let back = if tail.is_empty() { [] } else { self.back };
                (Queue { front= front, back= back, size= self.size - 1 }, Some(head))
            },
        };
    }
    public flow len(self) -> usize ![] { return self.size; }
    public flow is_empty(self) -> bool ![] { return self.size == 0; }
}

public type Deque<T> = { private front: List<T>, private back: List<T>, private size: usize };

impl Deque<T> {
    public flow new() -> Deque<T> ![] {
        return Deque { front= [], back= [], size= 0 };
    }
    flow balanced(self) -> Deque<T> ![] {
        if self.size < 2 || (!self.front.is_empty() && !self.back.is_empty()) { return self; }
        let forward = if self.front.is_empty() { self.back.reverse() } else { self.front };
        let midpoint = self.size / 2 + self.size % 2;
        var reversed_front: List<T> = [];
        var back: List<T> = [];
        var index: usize = 0;
        for value in forward {
            if index < midpoint { reversed_front = value :: reversed_front; }
            else { back = value :: back; }
            index = index + 1;
        }
        return Deque { front= reversed_front.reverse(), back= back, size= self.size };
    }
    public flow push_front(self, value: T) -> Deque<T> ![] {
        let updated = Deque { front= value :: self.front, back= self.back, size= self.size + 1 };
        return updated.balanced();
    }
    public flow push_back(self, value: T) -> Deque<T> ![] {
        let updated = Deque { front= self.front, back= value :: self.back, size= self.size + 1 };
        return updated.balanced();
    }
    public flow pop_front(self) -> (Deque<T>, Option<T>) ![] {
        return match self.front {
            [] => match self.back {
                [] => (self, None),
                value :: _ => (Deque { front= [], back= [], size= 0 }, Some(value)),
            },
            value :: tail => {
                let updated = Deque { front= tail, back= self.back, size= self.size - 1 };
                (updated.balanced(), Some(value))
            },
        };
    }
    public flow pop_back(self) -> (Deque<T>, Option<T>) ![] {
        return match self.back {
            [] => match self.front {
                [] => (self, None),
                value :: _ => (Deque { front= [], back= [], size= 0 }, Some(value)),
            },
            value :: tail => {
                let updated = Deque { front= self.front, back= tail, size= self.size - 1 };
                (updated.balanced(), Some(value))
            },
        };
    }
    public flow len(self) -> usize ![] { return self.size; }
    public flow is_empty(self) -> bool ![] { return self.size == 0; }
}
