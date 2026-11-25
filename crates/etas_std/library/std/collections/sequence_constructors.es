module std.collections;

impl Array<T> {
    public flow new() -> Array<T> ![] {
        return [];
    }
    public flow from_iter(items: Iter<T>) -> Array<T> ![] {
        var output: Array<T> = [];
        for value in items { output = output.push(value); }
        return output;
    }
    public flow init<effect E>(count: usize, f: (usize) -> T ![E]) -> Array<T> ![E] {
        var output: Array<T> = [];
        var index: usize = 0;
        while index < count {
            output = output.push(f(index));
            index = index + 1;
        }
        return output;
    }
    public flow repeat(count: usize, value: T) -> Array<T> ![] {
        var output: Array<T> = [];
        var index: usize = 0;
        while index < count {
            output = output.push(value);
            index = index + 1;
        }
        return output;
    }
    public flow first(self) -> Option<T> ![] {
        return self.get(0);
    }
    public flow last(self) -> Option<T> ![] {
        if self.length == 0 { return None; }
        return self.get(self.length - 1);
    }
    public flow split_at<I ~ Index>(self, index: I) -> (Slice<T>, Slice<T>) ![Error<std.runtime.error.IndexError>] {
        let offset = match index.to_usize() { Some(value) => value, None => usize.maximum() };
        if offset > self.length {
            perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= offset, length= self.length });
        }
        return (Slice { source= self, start= 0, length= offset }, Slice { source= self, start= offset, length= self.length - offset });
    }
    public flow retain<effect E>(self, predicate: (T) -> bool ![E]) -> Array<T> ![E] {
        var output: Array<T> = [];
        for value in self {
            if predicate(value) { output = output.push(value); }
        }
        return output;
    }
    public flow insert<I ~ Index>(self, index: I, value: T) -> Array<T> ![Error<std.runtime.error.IndexError>] {
        let offset = match index.to_usize() { Some(value) => value, None => usize.maximum() };
        if offset > self.length {
            perform Error.raise<std.runtime.error.IndexError>(std.runtime.error.IndexError { index= offset, length= self.length });
        }
        var output: Array<T> = [];
        var current: usize = 0;
        for item in self {
            if current == offset { output = output.push(value); }
            output = output.push(item);
            current = current + 1;
        }
        if offset == self.length { output = output.push(value); }
        return output;
    }
    public flow remove<I ~ Index>(self, index: I) -> (Array<T>, T) ![Error<std.runtime.error.IndexError>] {
        let removed = self.at(index);
        let offset = index.to_usize().unwrap();
        var output: Array<T> = [];
        var current: usize = 0;
        for value in self {
            if current != offset { output = output.push(value); }
            current = current + 1;
        }
        return (output, removed);
    }
    public flow reverse(self) -> Array<T> ![] {
        var output: Array<T> = [];
        var remaining = self.length;
        while remaining > 0 {
            remaining = remaining - 1;
            output = output.push(self.get(remaining).unwrap());
        }
        return output;
    }
    public flow clear(self) -> Array<T> ![] {
        return [];
    }
}
impl List<T> {
    public flow new() -> List<T> ![] {
        return [];
    }
    public flow from_iter(items: Iter<T>) -> List<T> ![] {
        var reversed: List<T> = [];
        for value in items { reversed = value :: reversed; }
        return reversed.reverse();
    }
    public flow clear(self) -> List<T> ![] {
        return [];
    }
}
impl Slice<T> {
    public flow first(self) -> Option<T> ![] {
        if self.length == 0 { return None; }
        return self.source.get(self.start);
    }
    public flow last(self) -> Option<T> ![] {
        if self.length == 0 { return None; }
        return self.source.get(self.start + self.length - 1);
    }
}
impl Stack<T> {
    public flow from_iter(items: Iter<T>) -> Stack<T> ![] {
        var reversed: List<T> = [];
        var size: usize = 0;
        for value in items { reversed = value :: reversed; size = size + 1; }
        return Stack { items= reversed.reverse(), size= size };
    }
    public flow peek(self) -> Option<T> ![] {
        return match self.items { [] => None, value :: _ => Some(value) };
    }
    public flow clear(self) -> Stack<T> ![] {
        return Stack { items= [], size= 0 };
    }
}
impl Queue<T> {
    public flow from_iter(items: Iter<T>) -> Queue<T> ![] {
        var reversed: List<T> = [];
        var size: usize = 0;
        for value in items { reversed = value :: reversed; size = size + 1; }
        return Queue { front= reversed.reverse(), back= [], size= size };
    }
    public flow peek(self) -> Option<T> ![] {
        return self.front.head();
    }
    public flow clear(self) -> Queue<T> ![] {
        return Queue { front= [], back= [], size= 0 };
    }
}
impl Deque<T> {
    public flow from_iter(items: Iter<T>) -> Deque<T> ![] {
        var reversed: List<T> = [];
        var size: usize = 0;
        for value in items { reversed = value :: reversed; size = size + 1; }
        let result = Deque { front= reversed.reverse(), back= [], size= size };
        return result.balanced();
    }
    public flow peek_front(self) -> Option<T> ![] {
        return if self.front.is_empty() { self.back.head() } else { self.front.head() };
    }
    public flow peek_back(self) -> Option<T> ![] {
        return if self.back.is_empty() { self.front.head() } else { self.back.head() };
    }
    public flow clear(self) -> Deque<T> ![] {
        return Deque { front= [], back= [], size= 0 };
    }
}
