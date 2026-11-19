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
        return Queue { front= self.front, back= value :: self.back, size= self.size + 1 };
    }

    public flow pop(self) -> (Queue<T>, Option<T>) ![] {
        if self.size == 0 {
            return (self, None);
        }
        var front = self.front;
        var back = self.back;
        if front.is_empty() {
            for value in back limit Iterations(18446744073709551615) {
                front = value :: front;
            }
            back = [];
        }
        return match front {
            head :: tail => (Queue { front= tail, back= back, size= self.size - 1 }, Some(head)),
            [] => std.core.abort("queue size invariant"),
        };
    }

    public flow len(self) -> usize ![] {
        return self.size;
    }

    public flow is_empty(self) -> bool ![] {
        return self.size == 0;
    }
}

public type Deque<T> = { private front: List<T>, private back: List<T>, private size: usize };

impl Deque<T> {
    public flow new() -> Deque<T> ![] {
        return Deque { front= [], back= [], size= 0 };
    }

    public flow push_front(self, value: T) -> Deque<T> ![] {
        return Deque { front= value :: self.front, back= self.back, size= self.size + 1 };
    }

    public flow push_back(self, value: T) -> Deque<T> ![] {
        return Deque { front= self.front, back= value :: self.back, size= self.size + 1 };
    }

    public flow pop_front(self) -> (Deque<T>, Option<T>) ![] {
        if self.size == 0 {
            return (self, None);
        }
        var front = self.front;
        var back = self.back;
        if front.is_empty() {
            var forward: List<T> = [];
            for value in back limit Iterations(18446744073709551615) {
                forward = value :: forward;
            }
            let midpoint = self.size / 2 + self.size % 2;
            var reversed_front: List<T> = [];
            back = [];
            var index: usize = 0;
            for value in forward limit Iterations(18446744073709551615) {
                if index < midpoint {
                    reversed_front = value :: reversed_front;
                } else {
                    back = value :: back;
                }
                index = index + 1;
            }
            for value in reversed_front limit Iterations(18446744073709551615) {
                front = value :: front;
            }
        }
        return match front {
            head :: tail => (Deque { front= tail, back= back, size= self.size - 1 }, Some(head)),
            [] => std.core.abort("deque size invariant"),
        };
    }

    public flow pop_back(self) -> (Deque<T>, Option<T>) ![] {
        if self.size == 0 {
            return (self, None);
        }
        var front = self.front;
        var back = self.back;
        if back.is_empty() {
            var reversed: List<T> = [];
            for value in front limit Iterations(18446744073709551615) {
                reversed = value :: reversed;
            }
            let midpoint = self.size / 2 + self.size % 2;
            var reversed_back: List<T> = [];
            front = [];
            var index: usize = 0;
            for value in reversed limit Iterations(18446744073709551615) {
                if index < midpoint {
                    reversed_back = value :: reversed_back;
                } else {
                    front = value :: front;
                }
                index = index + 1;
            }
            for value in reversed_back limit Iterations(18446744073709551615) {
                back = value :: back;
            }
        }
        return match back {
            head :: tail => (Deque { front= front, back= tail, size= self.size - 1 }, Some(head)),
            [] => std.core.abort("deque size invariant"),
        };
    }

    public flow len(self) -> usize ![] {
        return self.size;
    }

    public flow is_empty(self) -> bool ![] {
        return self.size == 0;
    }
}
