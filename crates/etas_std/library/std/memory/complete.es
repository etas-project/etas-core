module std.memory;

public import std.memory.WriteCondition.Any;
public import std.memory.WriteCondition.Exists;
public import std.memory.WriteCondition.Missing;


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
public type MemoryRegion<S> = { private stable_id: string, private name: string };

// Typed persistent memory support type.
public type MemorySelection<V> = {
    private evaluate: (Option<u32>) -> Array<SelectionEntry> ![],
    private maximum: Option<u32>,
};

// Typed storage receipt evidence.
public type MemoryTombstone = { opaque: string };

// Typed persistent memory support type.
type TransactionHandle;
public type MemoryTransaction = { private handle: TransactionHandle };

// Typed persistent memory version token.
public type MemoryVersion = { opaque: string };

// Closed storage evidence.
public enum MemoryWriteChange {
    Deleted(std.memory.MemoryTombstone),
    Written(std.memory.MemoryVersion),
}

// Storage preparation contract.
public type MemoryWriteIntent<K, V> = {
    private store: Store<K, V>,
    private mutation: WriteRequest<K, V>,
    private operation: StorageOperationRef,
    private wire: string,
};

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
public type Store<K, V> = { private description: StoreDescription };

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

// Bind a typed persistent memory region handle.
public flow region<S>(stable_id: string, name: string) -> MemoryRegion<S> ![] {
    return MemoryRegion { stable_id= stable_id, name= name };
}

// Construct an opaque memory version token for compare-and-set operations.
public flow version(token: string) -> MemoryVersion ![] {
    return MemoryVersion { opaque= token };
}

impl MemorySelection<V> {
    public flow limit(self, limit: std.runtime.limits.Limit) -> MemorySelection<V> ![] {
        let count = match limit {
            std.runtime.limits.Limit.Iterations(value) => value,
            std.runtime.limits.Limit.Tokens(value) => value,
            std.runtime.limits.Limit.ContextTokens(value) => value,
            std.runtime.limits.Limit.Attempts(value) => value,
            _ => std.core.abort("memory selection limit requires a count limit"),
        };
        if count.bits > 4294967295 { std.core.abort("memory selection count exceeds u32 range"); }
        return MemorySelection { evaluate= self.evaluate, maximum= Some(u32 { bits= count.bits }) };
    }
}

impl Store<K, V> {
    public flow prepare_delete(self, key: K, condition: WriteCondition) -> MemoryWriteIntent<K, V> ![Error<StorageError>] {
        let context = StorageContext.current();
        self.description.validate();
        let key_data = match std.runtime.value.Data.capture(key) {
            Ok(data) => data,
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message }),
        };
        let condition_text = condition.token();
        let operation_key = context.operation_key();
        var fingerprint = MutationFingerprint.new(self.description, "delete", context.max_value_bytes);
        fingerprint.value(key_data);
        fingerprint.part(condition_text);
        let operation = StorageOperationRef { key= operation_key, fingerprint= fingerprint.finish() };
        let wire = IntentEnvelope.encode(self.description, std.runtime.value.Data.Variant("Delete", [key_data, condition.data()]), operation, context.max_value_bytes);
        return MemoryWriteIntent { store= self, mutation= WriteRequest.Delete(key, condition), operation= operation, wire= wire };
    }
    public flow prepare_put(self, key: K, value: V, condition: WriteCondition) -> MemoryWriteIntent<K, V> ![Error<StorageError>] {
        let context = StorageContext.current();
        self.description.validate();
        let key_data = match std.runtime.value.Data.capture(key) {
            Ok(data) => data,
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message }),
        };
        let value_data = match std.runtime.value.Data.capture(value) {
            Ok(data) => data,
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message }),
        };
        let condition_text = condition.token();
        let operation_key = context.operation_key();
        var fingerprint = MutationFingerprint.new(self.description, "put", context.max_value_bytes);
        fingerprint.value(key_data);
        fingerprint.part(condition_text);
        fingerprint.value(value_data);
        let operation = StorageOperationRef { key= operation_key, fingerprint= fingerprint.finish() };
        let wire = IntentEnvelope.encode(self.description, std.runtime.value.Data.Variant("Put", [key_data, value_data, condition.data()]), operation, context.max_value_bytes);
        return MemoryWriteIntent { store= self, mutation= WriteRequest.Put(key, value, condition), operation= operation, wire= wire };
    }

    public flow reconcile(self, operation: StorageOperationRef) -> ReconcileResult<MemoryWriteReceipt<K>, MemoryWriteRejection> ![Error<StorageError>] {
        operation.validate();
        return perform std.effects.Memory.reconcile(self, operation);
    }

    public flow clear(self) -> unit ![] {
        // Finish the fenced read before mutation. Only delete the captured
        // versions, so a concurrent replacement is never removed by clear.
        var snapshot: List<MemoryEntry<K, V>> = [];
        var cursor: Option<MemoryCursor> = None;
        while true limit Iterations(18446744073709551615) {
            let request: ReadRequest<K> = ReadRequest.Page(cursor, None);
            let response: ReadResult<K, V> = perform std.effects.Memory.read(self, request);
            let page = match response {
                ReadResult.Page(value) => value,
                _ => std.core.abort("scan result ABI invariant"),
            };
            for entry in page.entries limit Iterations(18446744073709551615) {
                snapshot = entry :: snapshot;
            }
            cursor = page.cursor;
            if cursor.is_none() { break; }
        }
        for entry in snapshot.reverse() limit Iterations(18446744073709551615) {
            let request: WriteRequest<K, V> = WriteRequest.Delete(entry.key, WriteCondition.Match(entry.version));
            perform std.effects.Memory.write(self, request);
        }
    }

    public flow contains(self, key: K) -> bool ![] {
        let request: ReadRequest<K> = ReadRequest.Get(key);
        let response: ReadResult<K, V> = perform std.effects.Memory.read(self, request);
        return match response {
            ReadResult.Entry(value) => value.is_some(),
            _ => std.core.abort("get result ABI invariant"),
        };
    }

    public flow delete(self, key: K) -> unit ![] {
        let request: WriteRequest<K, V> = WriteRequest.Delete(key, WriteCondition.Any);
        perform std.effects.Memory.write(self, request);
    }

    public flow delete_versioned(self, key: K, version: MemoryVersion) -> unit ![] {
        let request: WriteRequest<K, V> = WriteRequest.Delete(key, WriteCondition.Match(version));
        perform std.effects.Memory.write(self, request);
    }

    public flow get(self, key: K) -> Option<V> ![] {
        let request: ReadRequest<K> = ReadRequest.Get(key);
        let response: ReadResult<K, V> = perform std.effects.Memory.read(self, request);
        return match response {
            ReadResult.Entry(None) => None,
            ReadResult.Entry(Some(entry)) => Some(entry.value),
            _ => std.core.abort("get result ABI invariant"),
        };
    }

    public flow get_entry(self, key: K) -> Option<MemoryEntry<K, V>> ![] {
        let request: ReadRequest<K> = ReadRequest.Get(key);
        let response: ReadResult<K, V> = perform std.effects.Memory.read(self, request);
        return match response {
            ReadResult.Entry(value) => value,
            _ => std.core.abort("get result ABI invariant"),
        };
    }

    public flow insert(self, key: K, value: V) -> unit ![] {
        let request: WriteRequest<K, V> = WriteRequest.Put(key, value, WriteCondition.Missing);
        perform std.effects.Memory.write(self, request);
    }

    public flow keys(self) -> List<K> ![] {
        var keys: List<K> = [];
        var cursor: Option<MemoryCursor> = None;
        while true limit Iterations(18446744073709551615) {
            let request: ReadRequest<K> = ReadRequest.Page(cursor, None);
            let response: ReadResult<K, V> = perform std.effects.Memory.read(self, request);
            let page = match response {
                ReadResult.Page(value) => value,
                _ => std.core.abort("scan result ABI invariant"),
            };
            for entry in page.entries limit Iterations(18446744073709551615) {
                keys = entry.key :: keys;
            }
            cursor = page.cursor;
            if cursor.is_none() { break; }
        }
        return keys.reverse();
    }

    public flow page(self, cursor: Option<MemoryCursor>, limit: u32) -> MemoryPage<K, V> ![] {
        let request: ReadRequest<K> = ReadRequest.Page(cursor, Some(limit));
        let response: ReadResult<K, V> = perform std.effects.Memory.read(self, request);
        return match response {
            ReadResult.Page(value) => value,
            _ => std.core.abort("scan result ABI invariant"),
        };
    }

    public flow put(self, key: K, value: V) -> unit ![] {
        let request: WriteRequest<K, V> = WriteRequest.Put(key, value, WriteCondition.Any);
        perform std.effects.Memory.write(self, request);
    }

    public flow put_versioned(self, key: K, value: V, version: MemoryVersion) -> unit ![] {
        let request: WriteRequest<K, V> = WriteRequest.Put(key, value, WriteCondition.Match(version));
        perform std.effects.Memory.write(self, request);
    }

    public flow query<Q>(self, query: Q) -> MemorySelection<V> ![] {
        let store = self;
        return MemorySelection { maximum= None, evaluate= (maximum: Option<u32>) => {
            let predicate = match std.runtime.value.Data.capture(query) {
                Ok(value) => value,
                Err(error) => std.core.abort(error.message),
            };
            let request: ReadRequest<K> = ReadRequest.Selection(SelectionRequest.Query(predicate, maximum));
            let response: ReadResult<K, V> = perform std.effects.Memory.read(store, request);
            return match response {
                ReadResult.Selected(page) => page.entries,
                _ => std.core.abort("selection result ABI invariant"),
            };
        } };
    }
    public flow related_to<Q>(self, query: Q) -> MemorySelection<V> ![] {
        let store = self;
        return MemorySelection { maximum= None, evaluate= (maximum: Option<u32>) => {
            let data = match std.runtime.value.Data.capture(query) {
                Ok(value) => value,
                Err(error) => std.core.abort(error.message),
            };
            let embedding = data.embedding();
            let count = match maximum { None => 100, Some(0) => 1, Some(count) => count };
            let request: ReadRequest<K> = ReadRequest.Selection(SelectionRequest.Vector(embedding, count));
            let response: ReadResult<K, V> = perform std.effects.Memory.read(store, request);
            return match response {
                ReadResult.Selected(page) => page.entries,
                _ => std.core.abort("selection result ABI invariant"),
            };
        } };
    }
    public flow scan(self) -> MemorySelection<V> ![] {
        let store = self;
        return MemorySelection { maximum= None, evaluate= (maximum: Option<u32>) => {
            let context = StorageContext.current();
            var output: Array<SelectionEntry> = [];
            var cursor: Option<MemoryCursor> = None;
            var total: usize = 0;
            while true limit Iterations(context.max_scan_rows + 1) {
                let remaining = match maximum {
                    None => None,
                    Some(count) => {
                        if total.bits >= count.bits { return output; }
                        Some(u32 { bits= count.bits - total.bits })
                    },
                };
                let page_limit = match remaining {
                    None => u32 { bits= context.max_page_entries.bits },
                    Some(count) => if count.bits < context.max_page_entries.bits { count } else { u32 { bits= context.max_page_entries.bits } },
                };
                let request: ReadRequest<K> = ReadRequest.Selection(SelectionRequest.Scan(cursor, page_limit));
                let response: ReadResult<K, V> = perform std.effects.Memory.read(store, request);
                let page = match response {
                    ReadResult.Selected(value) => value,
                    _ => std.core.abort("selection scan result ABI invariant"),
                };
                for entry in page.entries limit Iterations(context.max_scan_rows - total) {
                    output = output.push(entry);
                    total = total + 1;
                }
                cursor = page.cursor;
                if cursor.is_none() { return output; }
            }
            return std.core.abort("memory scan iteration limit exceeded");
        } };
    }
    public flow select<Q>(self, query: Q) -> MemorySelection<V> ![] {
        let store = self;
        return MemorySelection { maximum= None, evaluate= (maximum: Option<u32>) => {
            let predicate = match std.runtime.value.Data.capture(query) {
                Ok(value) => value,
                Err(error) => std.core.abort(error.message),
            };
            let request: ReadRequest<K> = ReadRequest.Selection(SelectionRequest.Query(predicate, maximum));
            let response: ReadResult<K, V> = perform std.effects.Memory.read(store, request);
            return match response {
                ReadResult.Selected(page) => page.entries,
                _ => std.core.abort("selection result ABI invariant"),
            };
        } };
    }

    public flow update(self, key: K, value: V) -> unit ![] {
        let request: WriteRequest<K, V> = WriteRequest.Put(key, value, WriteCondition.Exists);
        perform std.effects.Memory.write(self, request);
    }

    public flow upsert(self, key: K, value: V) -> unit ![] {
        let request: WriteRequest<K, V> = WriteRequest.Put(key, value, WriteCondition.Any);
        perform std.effects.Memory.write(self, request);
    }
}

impl MemoryWriteIntent<K, V> {
    public flow operation_ref(self) -> StorageOperationRef ![] {
        return self.operation;
    }

    public flow commit(self) -> WriteOutcome<MemoryWriteReceipt<K>, MemoryWriteRejection> ![Error<StorageError>] {
        return perform std.effects.Memory.commit(self.store, self.operation, self.mutation, self.wire);
    }
}
