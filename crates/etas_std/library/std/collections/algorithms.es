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
        if self.length == 0 { return (self, None); }
        let end = self.length - 1;
        let last = self.get(end);
        return (ArrayStorage.truncate(self, end), last);
    }

    public flow extend(self, appended: Array<T>) -> Array<T> ![] {
        return self + appended;
    }
}

impl List<T> {
    public flow len(self) -> usize ![] {
        var count: usize = 0;
        var cursor = self;
        while true {
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
        while true {
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
        for value in prepended {
            reversed = value :: reversed;
        }
        var output = self;
        for value in reversed {
            output = value :: output;
        }
        return output;
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
        while index < self.length {
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
        for value in self {
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

// Operations on the backing of an immutable array snapshot.
type ArrayStorage = {};
impl ArrayStorage {
    @std.role("std.collections.ArrayStorage.truncate")
    flow truncate<T>(items: Array<T>, length: usize) -> Array<T> ![];

    @std.role("std.collections.ArrayStorage.cached_map_index")
    flow cached_map_index<T>(slots: Array<T>, seed: u64) -> Array<usize> ![];

    @std.role("std.collections.ArrayStorage.remember_map_index")
    flow remember_map_index<T>(slots: Array<T>, seed: u64, index: Array<usize>) -> unit ![];
}
