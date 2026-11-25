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
    Selected(MemoryPage<K, V>),
}

public enum WriteRequest<K, V> {
    Put(K, V, WriteCondition),
    Delete(K, WriteCondition),
}

// Query payloads have explicit adapter language/version identity. The backend
// must reject an unregistered pair rather than generating an embedding.
public enum SelectionRequest {
    Query(string, u32, bytes, Option<u32>),
}
