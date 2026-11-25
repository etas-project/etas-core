module std.collections;

// Native recursive sum: [] constructs the empty value; :: constructs a cell.
public type List<T> = [] | T :: List<T>;

// [T] is the language's contiguous array value representation. The std name
// aliases that representation; its methods are implemented in Etas below.
public alias Array<T> = [T];

// Hash buckets index insertion-ordered live slots. Storage is private.
public type Map<K ~ Hash, V> = {
    private buckets: Array<List<usize>>,
    private slots: Array<Option<MapEntry<K, V>>>,
    private length: usize,
    private base: Hasher,
};
public type Set<T ~ Hash> = { private values: Map<T, unit> };
type MapEntry<K, V> = { key: K, value: V, hash: u64 };

public type Range<Position ~ Index> = {
    start: Position,
    end: Position,
    include_start: bool,
    include_end: bool,
};

public type Slice<T> = {
    private source: Array<T>,
    private start: usize,
    private length: usize,
};

public type Iter<T> = {
    next: () -> Option<(T, Iter<T>)> ![],
};
