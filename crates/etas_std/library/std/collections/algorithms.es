module std.collections;

impl Array<T> {
    public flow len(self) -> usize ![] {
        return self.length;
    }

    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }

    public flow get<I ~ Index>(self, index: I) -> Option<T> ![] {
        return match index.to_usize() {
            Some(offset) => {
                return match self[offset]? {
                    Ok(value) => Some(value),
                    Err(_) => None,
                };
            },
            None => None,
        };
    }

    public flow at<I ~ Index>(self, index: I) -> T ![Error<std.runtime.error.IndexError>] {
        return match self.get(index) {
            Some(value) => value,
            None => perform Error.raise<std.runtime.error.IndexError>(
                std.runtime.error.IndexError {
                    index= match index.to_usize() { Some(offset) => offset, None => usize.maximum() },
                    length= self.length,
                },
            ),
        };
    }

    public flow push(self, value: T) -> Array<T> ![] {
        return self + [value];
    }

    public flow pop(self) -> (Array<T>, Option<T>) ![] {
        var remaining: Array<T> = [];
        var previous: Option<T> = None;
        for value in self limit Iterations(18446744073709551615) {
            match previous {
                Some(last) => { remaining = remaining + [last]; },
                None => {},
            };
            previous = Some(value);
        }
        return (remaining, previous);
    }

    public flow extend(self, appended: Array<T>) -> Array<T> ![] {
        return self + appended;
    }
}

impl List<T> {
    public flow len(self) -> usize ![] {
        var count: usize = 0;
        var cursor = self;
        while true limit Iterations(18446744073709551615) {
            match cursor {
                [] => { return count; },
                _ :: tail => {
                    count = count + 1;
                    cursor = tail;
                },
            };
        }
        return std.core.abort("list traversal limit");
    }

    public flow is_empty(self) -> bool ![] {
        return match self {
            [] => true,
            _ :: _ => false,
        };
    }

    public flow get<I ~ Index>(self, index: I) -> Option<T> ![] {
        var offset = match index.to_usize() {
            Some(value) => value,
            None => { return None; },
        };
        var cursor = self;
        while true limit Iterations(18446744073709551615) {
            match cursor {
                [] => { return None; },
                head :: tail => {
                    if offset == 0 {
                        return Some(head);
                    }
                    offset = offset - 1;
                    cursor = tail;
                },
            };
        }
        return std.core.abort("list traversal limit");
    }

    public flow at<I ~ Index>(self, index: I) -> T ![Error<std.runtime.error.IndexError>] {
        return match self.get(index) {
            Some(value) => value,
            None => perform Error.raise<std.runtime.error.IndexError>(
                std.runtime.error.IndexError {
                    index= match index.to_usize() { Some(offset) => offset, None => usize.maximum() },
                    length= self.len(),
                },
            ),
        };
    }

    public flow head(self) -> Option<T> ![] {
        return match self {
            [] => None,
            head :: _ => Some(head),
        };
    }

    public flow tail(self) -> Option<List<T>> ![] {
        return match self {
            [] => None,
            _ :: tail => Some(tail),
        };
    }

    public flow pop(self) -> (List<T>, Option<T>) ![] {
        return match self {
            [] => (self, None),
            head :: tail => (tail, Some(head)),
        };
    }

    public flow push(self, value: T) -> List<T> ![] {
        return value :: self;
    }

    public flow extend(self, prepended: List<T>) -> List<T> ![] {
        var reversed: List<T> = [];
        for value in prepended limit Iterations(18446744073709551615) {
            reversed = value :: reversed;
        }
        var output = self;
        for value in reversed limit Iterations(18446744073709551615) {
            output = value :: output;
        }
        return output;
    }
}

impl Map<K, V> {
    public flow new() -> Map<K, V> ![] {
        return Map { entries= [], length= 0 };
    }

    public flow from_entries(items: List<(K, V)>) -> Map<K, V> ![] {
        var output: Map<K, V> = Map { entries= [], length= 0 };
        for entry in items limit Iterations(18446744073709551615) {
            let (key, value) = entry;
            output = output.insert(key, value);
        }
        return output;
    }

    public flow len(self) -> usize ![] {
        return self.length;
    }

    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }

    public flow get(self, key: K) -> Option<V> ![] {
        var cursor = self.entries;
        while true limit Iterations(18446744073709551615) {
            match cursor {
                [] => { return None; },
                entry :: tail => {
                    let (entry_key, value) = entry;
                    if entry_key == key {
                        return Some(value);
                    }
                    cursor = tail;
                },
            };
        }
        return std.core.abort("map traversal limit");
    }

    public flow contains_key(self, key: K) -> bool ![] {
        return match self.get(key) {
            Some(_) => true,
            None => false,
        };
    }

    public flow insert(self, key: K, value: V) -> Map<K, V> ![] {
        var reversed: List<(K, V)> = [];
        var replaced = false;
        for entry in self.entries limit Iterations(18446744073709551615) {
            let (entry_key, entry_value) = entry;
            if entry_key == key {
                reversed = (entry_key, value) :: reversed;
                replaced = true;
            } else {
                reversed = (entry_key, entry_value) :: reversed;
            }
        }
        if !replaced {
            reversed = (key, value) :: reversed;
        }
        return Map {
            entries= reversed.reverse(),
            length= self.length + if replaced { 0 } else { 1 },
        };
    }

    public flow remove(self, key: K) -> Map<K, V> ![] {
        var reversed: List<(K, V)> = [];
        var length: usize = 0;
        for entry in self.entries limit Iterations(18446744073709551615) {
            let (entry_key, entry_value) = entry;
            if entry_key != key {
                reversed = (entry_key, entry_value) :: reversed;
                length = length + 1;
            }
        }
        return Map { entries= reversed.reverse(), length= length };
    }
}

impl Slice<T> {
    public flow len(self) -> usize ![] {
        return self.length;
    }

    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }

    public flow at<I ~ Index>(self, index: I) -> T ![Error<std.runtime.error.IndexError>] {
        return match self.get(index) {
            Some(value) => value,
            None => perform Error.raise<std.runtime.error.IndexError>(
                std.runtime.error.IndexError {
                    index= match index.to_usize() { Some(offset) => offset, None => usize.maximum() },
                    length= self.length,
                },
            ),
        };
    }

    public flow get<I ~ Index>(self, index: I) -> Option<T> ![] {
        return match index.to_usize() {
            Some(offset) => {
                if offset < self.length {
                    return self.source.get(self.start + offset);
                }
                return None;
            },
            None => None,
        };
    }

    public flow to_array(self) -> Array<T> ![] {
        var output: Array<T> = [];
        var index: usize = 0;
        while index < self.length limit Iterations(18446744073709551615) {
            output = output.push(self.source.get(self.start + index).unwrap());
            index = index + 1;
        }
        return output;
    }
}

impl Range<T> {
    public flow closed(start: T, end: T) -> Range<T> ![] {
        return Range { start= start, end= end, include_start= true, include_end= true };
    }

    public flow open(start: T, end: T) -> Range<T> ![] {
        return Range { start= start, end= end, include_start= false, include_end= false };
    }
}

impl string {
    public flow len(self) -> usize ![] {
        return self.length;
    }

    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }
}

impl bytes {
    public flow len(self) -> usize ![] {
        return self.length;
    }

    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }
}

impl Set<T> {
    public flow new() -> Set<T> ![] {
        return Set { elements= [], length= 0 };
    }

    public flow from_list(items: List<T>) -> Set<T> ![] {
        var output: Set<T> = Set { elements= [], length= 0 };
        for value in items limit Iterations(18446744073709551615) {
            if !output.contains(value) {
                output = Set { elements= value :: output.elements, length= output.length + 1 };
            }
        }
        return Set { elements= output.elements.reverse(), length= output.length };
    }

    public flow len(self) -> usize ![] {
        return self.length;
    }

    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }

    public flow contains(self, value: T) -> bool ![] {
        for element in self.elements limit Iterations(18446744073709551615) {
            if element == value {
                return true;
            }
        }
        return false;
    }

    public flow insert(self, value: T) -> Set<T> ![] {
        if self.contains(value) {
            return self;
        }
        let reversed = value :: (self.elements.reverse());
        return Set { elements= reversed.reverse(), length= self.length + 1 };
    }

    public flow remove(self, value: T) -> Set<T> ![] {
        var reversed: List<T> = [];
        var length: usize = 0;
        for element in self.elements limit Iterations(18446744073709551615) {
            if element != value {
                reversed = element :: reversed;
                length = length + 1;
            }
        }
        return Set { elements= reversed.reverse(), length= length };
    }
}

impl bytes {
    public flow get<I ~ Index>(self, index: I) -> Option<u8> ![] {
        return match self[index]? {
            Ok(value) => Some(value),
            Err(_) => None,
        };
    }
}

impl List<T> {
    public flow reverse(self) -> List<T> ![] {
        var output: List<T> = [];
        for value in self limit Iterations(18446744073709551615) {
            output = value :: output;
        }
        return output;
    }
}

impl Array<T> {
    public flow set<I ~ Index>(self, index: I, value: T) -> Array<T> ![Error<std.runtime.error.IndexError>] {
        let offset = match index.to_usize() { Some(offset) => offset, None => usize.maximum() };
        if offset >= self.length {
            return perform Error.raise<std.runtime.error.IndexError>(
                std.runtime.error.IndexError { index= offset, length= self.length },
            );
        }
        var output = self;
        output[offset] = value;
        return output;
    }
}
