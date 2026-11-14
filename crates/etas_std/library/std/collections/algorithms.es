module std.collections;
import std.collections.array_push;

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
