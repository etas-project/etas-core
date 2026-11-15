module std.collections;

@std.role("std.collections.Array")
public type Array<Element>;

@std.role("std.collections.List")
public type List<Element>;

@std.role("std.collections.Map")
public type Map<Key, Value>;

@std.role("std.collections.Set")
public type Set<Element>;

@std.role("std.collections.Range")
public type Range<Position>;

@std.role("std.collections.Slice")
public type Slice<Element>;

@std.role("std.collections.Iter")
public type Iter<Element>;
