module std.memory;

public import std.memory.WriteCondition.Any;
public import std.memory.WriteCondition.Exists;
public import std.memory.WriteCondition.Missing;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.


// Closed storage outcome.
public enum ConfirmedOutcome<R, N> {
    Committed(R),
    NotCommitted(std.memory.StorageOperationRef, N),
}


// Typed persistent memory conflict information.
public type MemoryConflict = { expected: std.option.Option<std.memory.MemoryVersion>, actual: std.option.Option<std.memory.MemoryVersion>, current_value: std.option.Option<std.json.JsonValue> };

// Store-scoped continuation token; writes invalidate the scan cursor.
public type MemoryCursor = { opaque: string };

// A typed stored value and its compare-and-set version.
public type MemoryEntry<K, V> = { key: K, value: V, version: std.memory.MemoryVersion };

// A bounded page of entries and an optional continuation cursor.
public type MemoryPage<K, V> = { entries: std.collections.List<std.memory.MemoryEntry<K, V>>, cursor: std.option.Option<std.memory.MemoryCursor> };

// Typed persistent memory support type.
@std.role("std.memory.MemoryRegion")
public type MemoryRegion<S>;

// Typed persistent memory support type.
@std.role("std.memory.MemorySelection")
public type MemorySelection<V>;

// Typed storage receipt evidence.
public type MemoryTombstone = { opaque: string };

// Typed persistent memory support type.
public type MemoryTransaction;

// Typed persistent memory version token.
public type MemoryVersion = { opaque: string };

// Closed storage evidence.
public enum MemoryWriteChange {
    Deleted(std.memory.MemoryTombstone),
    Written(std.memory.MemoryVersion),
}

// Storage preparation contract.
@std.role("std.memory.MemoryWriteIntent")
public type MemoryWriteIntent<K, V>;

// Typed storage receipt evidence.
public type MemoryWriteReceipt<K> = { operation: std.memory.StorageOperationRef, target: std.memory.MemoryWriteTarget<K>, change: std.memory.MemoryWriteChange, durability: std.memory.StorageDurability };

// Closed storage evidence.
public enum MemoryWriteRejection {
    ConditionConflict(std.option.Option<std.memory.MemoryVersion>, std.option.Option<std.memory.MemoryVersion>),
    Rejected(std.memory.StorageError),
    Unchanged,
}

// Typed storage receipt evidence.
public type MemoryWriteTarget<K> = { region: string, store: std.collections.Array<string>, schema_fingerprint: std.option.Option<string>, key: K };


// Closed storage outcome.
public enum ReconcileResult<R, N> {
    Expired,
    Found(std.memory.ConfirmedOutcome<R, N>),
    Unresolved,
}

// Closed storage evidence.
public enum StorageDurability {
    SqliteWalFull,
    Volatile,
}

// Storage preparation contract.
public type StorageError = { code: string, message: string };

// Storage preparation contract.
public type StorageOperationRef = { key: string, fingerprint: string };

// Typed persistent memory support type.
@std.role("std.memory.Store")
public type Store<K, V>;

// Conditional mutation requirement.
public enum WriteCondition {
    Any,
    Exists,
    Match(std.memory.MemoryVersion),
    Missing,
}

// Closed storage outcome.
public enum WriteOutcome<R, N> {
    Committed(R),
    NotCommitted(std.memory.StorageOperationRef, N),
    Unknown(std.memory.StorageOperationRef),
}

// Clear a persistent store.
@std.binding("std.memory.clear")
public flow clear<K, V>(arg0: std.memory.Store<K, V>) -> unit ![] {}

// Commit an immutable storage intent or query its retained completion evidence.
@std.binding("std.memory.commit")
public flow commit<K, V>(arg0: std.memory.MemoryWriteIntent<K, V>) -> std.memory.WriteOutcome<std.memory.MemoryWriteReceipt<K>, std.memory.MemoryWriteRejection> ![Error<std.memory.StorageError>] {}

// Return whether a persistent store contains the given key.
@std.binding("std.memory.contains")
public flow contains<K, V>(arg0: std.memory.Store<K, V>, arg1: K) -> bool ![] {}

// Delete a typed value from a persistent store.
@std.binding("std.memory.delete")
public flow delete<K, V>(arg0: std.memory.Store<K, V>, arg1: K) -> unit ![] {}

// Delete a typed value if the persistent store entry still has the expected version.
@std.binding("std.memory.delete_versioned")
public flow delete_versioned<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: std.memory.MemoryVersion) -> unit ![] {}

// Read a typed value from a persistent store.
@std.binding("std.memory.get")
public flow get<K, V>(arg0: std.memory.Store<K, V>, arg1: K) -> std.option.Option<V> ![] {}

// Read the key, value and version atomically for a conditional write.
@std.binding("std.memory.get_entry")
public flow get_entry<K, V>(arg0: std.memory.Store<K, V>, arg1: K) -> std.option.Option<std.memory.MemoryEntry<K, V>> ![] {}

// Insert a typed value into a persistent store.
@std.binding("std.memory.insert")
public flow insert<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: V) -> unit ![] {}

// List keys from a persistent store.
@std.binding("std.memory.keys")
public flow keys<K, V>(arg0: std.memory.Store<K, V>) -> std.collections.List<K> ![] {}

// Limit a typed persistent-memory selection without performing additional host actions.
@std.binding("std.memory.limit")
public flow limit<V>(arg0: std.memory.MemorySelection<V>, arg1: std.runtime.limits.Limit) -> std.memory.MemorySelection<V> ![] {}

// Prepare immutable write data or inspect its identity without accessing storage.
@std.binding("std.memory.operation_ref")
public flow operation_ref<K, V>(arg0: std.memory.MemoryWriteIntent<K, V>) -> std.memory.StorageOperationRef ![] {}

// Read one bounded page; pass None for the first page. Writes invalidate its cursor.
@std.binding("std.memory.page")
public flow page<K, V>(arg0: std.memory.Store<K, V>, arg1: std.option.Option<std.memory.MemoryCursor>, arg2: u32) -> std.memory.MemoryPage<K, V> ![] {}

// Prepare immutable write data or inspect its identity without accessing storage.
@std.binding("std.memory.prepare_delete")
public flow prepare_delete<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: std.memory.WriteCondition) -> std.memory.MemoryWriteIntent<K, V> ![Error<std.memory.StorageError>] {}

// Prepare immutable write data or inspect its identity without accessing storage.
@std.binding("std.memory.prepare_put")
public flow prepare_put<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: V, arg3: std.memory.WriteCondition) -> std.memory.MemoryWriteIntent<K, V> ![Error<std.memory.StorageError>] {}

// Write a typed value into a persistent store.
@std.binding("std.memory.put")
public flow put<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: V) -> unit ![] {}

// Write a typed value if the persistent store entry still has the expected version.
@std.binding("std.memory.put_versioned")
public flow put_versioned<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: V, arg3: std.memory.MemoryVersion) -> unit ![] {}

// Build a typed persistent-memory query from a store.
@std.binding("std.memory.query")
public flow query<K, V, Q>(arg0: std.memory.Store<K, V>, arg1: Q) -> std.memory.MemorySelection<V> ![] {}

// Commit an immutable storage intent or query its retained completion evidence.
@std.binding("std.memory.reconcile")
public flow reconcile<K, V>(arg0: std.memory.Store<K, V>, arg1: std.memory.StorageOperationRef) -> std.memory.ReconcileResult<std.memory.MemoryWriteReceipt<K>, std.memory.MemoryWriteRejection> ![Error<std.memory.StorageError>] {}

// Bind a typed persistent memory region handle.
@std.binding("std.memory.region")
public flow region<S>(arg0: string, arg1: string) -> std.memory.MemoryRegion<S> ![] {}

// Build a retrieval-oriented memory selection related to the given query value.
@std.binding("std.memory.related_to")
public flow related_to<K, V, Q>(arg0: std.memory.Store<K, V>, arg1: Q) -> std.memory.MemorySelection<V> ![] {}

// Build a typed persistent-memory scan over a store.
@std.binding("std.memory.scan")
public flow scan<K, V>(arg0: std.memory.Store<K, V>) -> std.memory.MemorySelection<V> ![] {}

// Build a typed persistent-memory selection from a store.
@std.binding("std.memory.select")
public flow select<K, V, Q>(arg0: std.memory.Store<K, V>, arg1: Q) -> std.memory.MemorySelection<V> ![] {}

// Update a typed value in a persistent store.
@std.binding("std.memory.update")
public flow update<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: V) -> unit ![] {}

// Upsert a typed value in a persistent store.
@std.binding("std.memory.upsert")
public flow upsert<K, V>(arg0: std.memory.Store<K, V>, arg1: K, arg2: V) -> unit ![] {}

// Construct an opaque memory version token for compare-and-set operations.
@std.binding("std.memory.version")
public flow version(arg0: string) -> std.memory.MemoryVersion ![] {}

impl std.memory.MemorySelection<V> {
    public flow limit(self, arg0: std.runtime.limits.Limit) -> std.memory.MemorySelection<V> ![] { return std.memory.limit(self, arg0); }
}

impl std.memory.Store<K, V> {
    public flow clear(self) -> unit ![] { return std.memory.clear(self); }
    public flow contains(self, arg0: K) -> bool ![] { return std.memory.contains(self, arg0); }
    public flow delete(self, arg0: K) -> unit ![] { return std.memory.delete(self, arg0); }
    public flow delete_versioned(self, arg0: K, arg1: std.memory.MemoryVersion) -> unit ![] { return std.memory.delete_versioned(self, arg0, arg1); }
    public flow get(self, arg0: K) -> std.option.Option<V> ![] { return std.memory.get(self, arg0); }
    public flow get_entry(self, arg0: K) -> std.option.Option<std.memory.MemoryEntry<K, V>> ![] { return std.memory.get_entry(self, arg0); }
    public flow insert(self, arg0: K, arg1: V) -> unit ![] { return std.memory.insert(self, arg0, arg1); }
    public flow keys(self) -> std.collections.List<K> ![] { return std.memory.keys(self); }
    public flow page(self, arg0: std.option.Option<std.memory.MemoryCursor>, arg1: u32) -> std.memory.MemoryPage<K, V> ![] { return std.memory.page(self, arg0, arg1); }
    public flow put(self, arg0: K, arg1: V) -> unit ![] { return std.memory.put(self, arg0, arg1); }
    public flow put_versioned(self, arg0: K, arg1: V, arg2: std.memory.MemoryVersion) -> unit ![] { return std.memory.put_versioned(self, arg0, arg1, arg2); }
    public flow query<Q>(self, arg0: Q) -> std.memory.MemorySelection<V> ![] { return std.memory.query(self, arg0); }
    public flow related_to<Q>(self, arg0: Q) -> std.memory.MemorySelection<V> ![] { return std.memory.related_to(self, arg0); }
    public flow scan(self) -> std.memory.MemorySelection<V> ![] { return std.memory.scan(self); }
    public flow select<Q>(self, arg0: Q) -> std.memory.MemorySelection<V> ![] { return std.memory.select(self, arg0); }
    public flow update(self, arg0: K, arg1: V) -> unit ![] { return std.memory.update(self, arg0, arg1); }
    public flow upsert(self, arg0: K, arg1: V) -> unit ![] { return std.memory.upsert(self, arg0, arg1); }
}
