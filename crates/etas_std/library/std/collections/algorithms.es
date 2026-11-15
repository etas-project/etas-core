module std.collections;
import std.collections.{array_get, array_push};

public flow array_pop<T>(values: Array<T>) -> (Array<T>, Option<T>) ![] {
    var remaining: Array<T> = [];
    var previous: Option<T> = None;
    for value in values limit Iterations(18446744073709551615) {
        match previous {
            Some(last) => { remaining = array_push(remaining, last); },
            None => {},
        };
        previous = Some(value);
    }
    return (remaining, previous);
}

public flow list_head<T>(values: List<T>) -> Option<T> ![] {
    for value in values limit Iterations(18446744073709551615) {
        return Some(value);
    }
    return None;
}

flow reversed<T>(values: List<T>) -> List<T> ![] {
    var output: List<T> = [];
    for value in values limit Iterations(18446744073709551615) {
        output = value :: output;
    }
    return output;
}

public flow list_pop<T>(values: List<T>) -> (List<T>, Option<T>) ![] {
    var remaining: List<T> = [];
    var first: Option<T> = None;
    var started = false;
    for value in values limit Iterations(18446744073709551615) {
        if started {
            remaining = value :: remaining;
        } else {
            first = Some(value);
            started = true;
        }
    }
    return (reversed(remaining), first);
}

public flow list_tail<T>(values: List<T>) -> Option<List<T>> ![] {
    let (remaining, first) = list_pop(values);
    return match first { Some(_) => Some(remaining), None => None };
}

public flow array_extend<T>(values: Array<T>, appended: Array<T>) -> Array<T> ![] {
    var output = values;
    for value in appended limit Iterations(18446744073709551615) {
        output = array_push(output, value);
    }
    return output;
}

public flow list_push<T>(values: List<T>, value: T) -> List<T> ![] {
    return value :: values;
}

// List.extend prepends the second list, preserving its order.
public flow list_extend<T>(values: List<T>, prepended: List<T>) -> List<T> ![] {
    var reversed: List<T> = [];
    for value in prepended limit Iterations(18446744073709551615) {
        reversed = value :: reversed;
    }
    var output = values;
    for value in reversed limit Iterations(18446744073709551615) {
        output = value :: output;
    }
    return output;
}

impl Array<T> {
    public flow get<I ~ Index>(self, index: I) -> Option<T> ![] {
        return array_get(self, index);
    }

    public flow push(self, value: T) -> Array<T> ![] {
        return array_push(self, value);
    }

    public flow pop(self) -> (Array<T>, Option<T>) ![] {
        return array_pop(self);
    }

    public flow extend(self, appended: Array<T>) -> Array<T> ![] {
        return array_extend(self, appended);
    }
}

impl List<T> {
    public flow head(self) -> Option<T> ![] { return list_head(self); }
    public flow tail(self) -> Option<List<T>> ![] { return list_tail(self); }
    public flow pop(self) -> (List<T>, Option<T>) ![] { return list_pop(self); }

    public flow push(self, value: T) -> List<T> ![] {
        return list_push(self, value);
    }

    public flow extend(self, prepended: List<T>) -> List<T> ![] {
        return list_extend(self, prepended);
    }
}
