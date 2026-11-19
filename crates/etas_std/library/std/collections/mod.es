module std.collections;

// Native recursive sum: [] constructs the empty value; :: constructs a cell.
public type List<T> = [] | T :: List<T>;

// [T] is the language's contiguous array value representation. The std name
// aliases that representation; its methods are implemented in Etas below.
public alias Array<T> = [T];

// Insertion order is retained in the value. Key equality and collection
// operations are implemented by the owning impl, not by a runtime map object.
public type Map<K, V> = {
    private entries: List<(K, V)>,
    length: usize,
};

public type Set<T> = {
    private elements: List<T>,
    length: usize,
};

public type Range<Position ~ Index> = {
    start: Position,
    end: Position,
    include_start: bool,
    include_end: bool,
};

public type Slice<T> = {
    private source: Array<T>,
    private start: usize,
    length: usize,
};

public type Iter<T> = {
    next: () -> Option<(T, Iter<T>)> ![],
};
