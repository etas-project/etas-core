module std.memory;

// Authority comes from an observed live place, not from its descriptive path.
type MemoryCapability;

public type MemoryPlace = {
    private capability: MemoryCapability,
    region: string,
    path: Array<string>,
};

// Source protocol for a single backend read or conditional mutation. Scanning
// across pages, key collection and clear policy are implemented in Store.
public enum ReadRequest<K> {
    Get(K),
    Page(Option<MemoryCursor>, Option<u32>),
    Selection(SelectionRequest),
}

public enum ReadResult<K, V> {
    Entry(Option<MemoryEntry<K, V>>),
    Page(MemoryPage<K, V>),
    Selected(SelectedPage),
}

public enum WriteRequest<K, V> {
    Put(K, V, WriteCondition),
    Delete(K, WriteCondition),
}

// Neutral backend data is checked against the captured Store<K,V> before it
// is exposed. Source turns these rows into prompt JSON, preserving key/version.
public type SelectionEntry = {
    key: std.runtime.value.Data,
    value: std.runtime.value.Data,
    version: MemoryVersion,
};

public type SelectedPage = { entries: Array<SelectionEntry>, cursor: Option<MemoryCursor> };

public enum SelectionRequest {
    Scan(Option<MemoryCursor>, u32),
    Query(std.runtime.value.Data, Option<u32>),
    Vector(Array<f32>, u32),
}
