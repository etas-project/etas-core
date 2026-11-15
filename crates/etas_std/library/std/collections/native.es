module std.collections;

import std.core.Index;

// Persistent array storage access and allocation are host primitives.
@std.binding("std.collections.array_get")
public flow array_get<T, I ~ Index>(values: Array<T>, index: I) -> Option<T> ![] {}

@std.binding("std.collections.array_push")
public flow array_push<T>(values: Array<T>, value: T) -> Array<T> ![] {}
