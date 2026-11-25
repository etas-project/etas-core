flow window<T>(items: Array<T>) -> Slice<T> ![Error<std.runtime.error.IndexError>] {
    return items[1, 4);
}

flow main(args: Array<string>) -> i32 ![] {
    let original: Array<i32> = [0, 1, 2, 3, 4];
    let view: Slice<i32> = window(original)?.unwrap();
    std.core.assert(view.len() == 3);
    std.core.assert(view.to_array() == [1, 2, 3]);
    let suffix: Slice<i32> = view[1, 3)?.unwrap();
    let first = suffix[0]?.unwrap();
    std.core.assert(first == 2);
    std.core.assert(suffix.first().unwrap() == 2);
    std.core.assert(suffix.last().unwrap() == 3);
    let (_, split) = original.split_at(2)?.unwrap();
    let equivalent = split[0, 2)?.unwrap();
    std.core.assert(suffix == equivalent);
    var total: i32 = 0;
    for value in view { total = total + value; }
    std.core.assert(total == 6);
    let changed = original.set(2, 99)?.unwrap();
    std.core.assert(changed.get(2).unwrap() == 99);
    std.core.assert(view.get(1).unwrap() == 2);
    return 0;
}
