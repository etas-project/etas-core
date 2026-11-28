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
    private evaluate: (Option<usize>) -> Array<V> ![Error<StorageError>],
    private maximum: Option<usize>,
    private content_bounds: Array<std.runtime.limits.Limit>,
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
    private mutation: WriteRequest<bytes, bytes>,
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
    Durable,
    Volatile,
}

// Storage preparation contract.
public type StorageError = { code: string, message: string, operation: Option<StorageOperationRef> };

// Storage preparation contract.
public type StorageOperationRef = { key: string, fingerprint: string };

// Typed persistent memory support type.
public type Store<K, V> = {
    private description: StoreDescription,
    private key_format: Option<StorageCodec<K>>,
    private value_format: Option<StorageCodec<V>>,
    private encoded_schema: Option<string>,
};

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
public flow region<S>(stable_id: string, store: string) -> MemoryRegion<S> ![] {
    return MemoryRegion { stable_id= stable_id, name= store };
}

// Construct an opaque memory version token for compare-and-set operations.
public flow version(token: string) -> MemoryVersion ![] {
    return MemoryVersion { opaque= token };
}

impl MemorySelection<V> {
    public flow take(self, count: usize) -> MemorySelection<V> ![] {
        let maximum = match self.maximum { None => count, Some(value) => if value < count { value } else { count } };
        return MemorySelection { evaluate= self.evaluate, maximum= Some(maximum), content_bounds= self.content_bounds };
    }

    public flow limit(self, bound: std.runtime.limits.Limit) -> MemorySelection<V> ![Error<StorageError>] {
        match bound {
            std.runtime.limits.Limit.Tokens(_) => {},
            std.runtime.limits.Limit.ContextTokens(_) => {},
            _ => perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "memory selection accepts only content token limits", operation= None }),
        };
        return MemorySelection { evaluate= self.evaluate, maximum= self.maximum, content_bounds= self.content_bounds.push(bound) };
    }

    public flow collect(self) -> Array<V> ![Error<StorageError>] {
        if self.content_bounds.length > 0 {
            perform Error.raise<StorageError>(StorageError { code= "measure_unavailable", message= "memory selection requires a resolved tokenizer measurement context", operation= None });
        }
        return (self.evaluate)(self.maximum);
    }
}

impl Store<K ~ StorageKey, V ~ StorageData> {
    public flow with_codecs(self, key: StorageCodec<K>, value: StorageCodec<V>) -> Store<K, V> ![] {
        return Store { description= self.description, key_format= Some(key), value_format= Some(value), encoded_schema= None };
    }

    flow key_encoding(self) -> StorageCodec<K> ![] {
        return match self.key_format { Some(value) => value, None => std.memory.key_codec<K>() };
    }

    flow value_encoding(self) -> StorageCodec<V> ![] {
        return match self.value_format { Some(value) => value, None => std.memory.codec<V>() };
    }

    flow codec_view(self) -> Store<K, V> ![Error<StorageError>] {
        if self.encoded_schema.is_some() { return self; }
        self.description.validate();
        let key = self.key_encoding();
        let value = self.value_encoding();
        key.validate();
        value.validate();
        let context = StorageContext.current();
        var schema = MutationFingerprint.new(self.description, "codec-view.v1", context.max_value_bytes);
        schema.part(StorageValue.schema<K>());
        schema.part(key.format);
        schema.part(std.text.from_u128(key.version.bits));
        schema.part(StorageValue.schema<V>());
        schema.part(value.format);
        schema.part(std.text.from_u128(value.version.bits));
        return Store { description= self.description, key_format= Some(key), value_format= Some(value), encoded_schema= Some(schema.finish_schema()) };
    }

    flow bound_description(self) -> StoreDescription ![] {
        return StoreDescription { region= self.description.region, path= self.description.path, schema_fingerprint= self.encoded_schema };
    }

    flow encode_key(self, key: K) -> bytes ![Error<StorageError>] {
        let format = self.key_encoding();
        return match (format.encode)(key) { Ok(value) => value, Err(error) => perform Error.raise<StorageError>(error) };
    }

    flow encode_value(self, value: V) -> bytes ![Error<StorageError>] {
        let format = self.value_encoding();
        return match (format.encode)(value) { Ok(value) => value, Err(error) => perform Error.raise<StorageError>(error) };
    }

    flow decode_key(self, input: bytes) -> K ![Error<StorageError>] {
        let format = self.key_encoding();
        return match (format.decode)(input) { Ok(value) => value, Err(error) => perform Error.raise<StorageError>(error) };
    }

    flow decode_value(self, input: bytes) -> V ![Error<StorageError>] {
        let format = self.value_encoding();
        return match (format.decode)(input) { Ok(value) => value, Err(error) => perform Error.raise<StorageError>(error) };
    }

    flow decode_receipt(self, receipt: MemoryWriteReceipt<bytes>) -> MemoryWriteReceipt<K> ![Error<StorageError>] {
        let target = receipt.target;
        return MemoryWriteReceipt {
            operation= receipt.operation,
            target= MemoryWriteTarget { region= target.region, store= target.store, schema_fingerprint= target.schema_fingerprint, key= self.decode_key(target.key) },
            change= receipt.change,
            durability= receipt.durability,
        };
    }

    flow decode_page(self, page: MemoryPage<bytes, bytes>) -> MemoryPage<K, V> ![Error<StorageError>] {
        var entries: List<MemoryEntry<K, V>> = [];
        for entry in page.entries {
            entries = MemoryEntry { key= self.decode_key(entry.key), value= self.decode_value(entry.value), version= entry.version } :: entries;
        }
        return MemoryPage { entries= entries.reverse(), cursor= page.cursor };
    }

    public flow prepare_delete(self, key: K, condition: WriteCondition) -> MemoryWriteIntent<K, V> ![Error<StorageError>] {
        let store = self.codec_view();
        let context = StorageContext.current();
        let key_bytes = store.encode_key(key);
        let condition_text = condition.token();
        let operation_key = context.operation_key();
        let description = store.bound_description();
        var fingerprint = MutationFingerprint.new(description, "delete", context.max_value_bytes);
        fingerprint.value(std.runtime.value.Data.Bytes(key_bytes));
        fingerprint.part(condition_text);
        let operation = StorageOperationRef { key= operation_key, fingerprint= fingerprint.finish() };
        let wire = IntentEnvelope.encode(description, std.runtime.value.Data.Variant("Delete", [std.runtime.value.Data.Bytes(key_bytes), condition.data()]), operation, context.max_value_bytes);
        return MemoryWriteIntent { store= store, mutation= WriteRequest.Delete(key_bytes, condition), operation= operation, wire= wire };
    }

    public flow prepare_put(self, key: K, value: V, condition: WriteCondition) -> MemoryWriteIntent<K, V> ![Error<StorageError>] {
        let store = self.codec_view();
        let context = StorageContext.current();
        let key_bytes = store.encode_key(key);
        let value_bytes = store.encode_value(value);
        let condition_text = condition.token();
        let operation_key = context.operation_key();
        let description = store.bound_description();
        var fingerprint = MutationFingerprint.new(description, "put", context.max_value_bytes);
        fingerprint.value(std.runtime.value.Data.Bytes(key_bytes));
        fingerprint.part(condition_text);
        fingerprint.value(std.runtime.value.Data.Bytes(value_bytes));
        let operation = StorageOperationRef { key= operation_key, fingerprint= fingerprint.finish() };
        let wire = IntentEnvelope.encode(description, std.runtime.value.Data.Variant("Put", [std.runtime.value.Data.Bytes(key_bytes), std.runtime.value.Data.Bytes(value_bytes), condition.data()]), operation, context.max_value_bytes);
        return MemoryWriteIntent { store= store, mutation= WriteRequest.Put(key_bytes, value_bytes, condition), operation= operation, wire= wire };
    }

    public flow reconcile(self, operation: StorageOperationRef) -> ReconcileResult<MemoryWriteReceipt<K>, MemoryWriteRejection> ![Error<StorageError>] {
        operation.validate();
        let store = self.codec_view();
        let result: ReconcileResult<MemoryWriteReceipt<bytes>, MemoryWriteRejection> = perform std.effects.Memory.reconcile(store, operation);
        return match result {
            ReconcileResult.Found(ConfirmedOutcome.Committed(receipt)) => ReconcileResult.Found(ConfirmedOutcome.Committed(store.decode_receipt(receipt))),
            ReconcileResult.Found(ConfirmedOutcome.NotCommitted(reference, reason)) => ReconcileResult.Found(ConfirmedOutcome.NotCommitted(reference, reason)),
            ReconcileResult.Unresolved => ReconcileResult.Unresolved,
            ReconcileResult.Expired => ReconcileResult.Expired,
        };
    }

    public flow get(self, key: K) -> Option<V> ![Error<StorageError>] {
        let store = self.codec_view();
        let request: ReadRequest<bytes> = ReadRequest.Get(store.encode_key(key));
        let response: ReadResult<bytes, bytes> = perform std.effects.Memory.read(store, request);
        return match response {
            ReadResult.Entry(None) => None,
            ReadResult.Entry(Some(entry)) => Some(store.decode_value(entry.value)),
            _ => std.core.abort("get result ABI invariant"),
        };
    }

    public flow get_entry(self, key: K) -> Option<MemoryEntry<K, V>> ![Error<StorageError>] {
        let store = self.codec_view();
        let request: ReadRequest<bytes> = ReadRequest.Get(store.encode_key(key));
        let response: ReadResult<bytes, bytes> = perform std.effects.Memory.read(store, request);
        return match response {
            ReadResult.Entry(None) => None,
            ReadResult.Entry(Some(entry)) => Some(MemoryEntry { key= store.decode_key(entry.key), value= store.decode_value(entry.value), version= entry.version }),
            _ => std.core.abort("get result ABI invariant"),
        };
    }

    public flow contains(self, key: K) -> bool ![Error<StorageError>] {
        let entry = self.get(key);
        return entry.is_some();
    }

    public flow page(self, cursor: Option<MemoryCursor>, page_size: u32) -> MemoryPage<K, V> ![Error<StorageError>] {
        let context = StorageContext.current();
        if page_size == 0 || page_size.bits > context.max_page_entries.bits {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "memory page size must be positive and within the configured page bound", operation= None });
        }
        let store = self.codec_view();
        return store.decode_page(store.raw_page(cursor, page_size));
    }

    flow raw_page(self, cursor: Option<MemoryCursor>, page_size: u32) -> MemoryPage<bytes, bytes> ![Error<StorageError>] {
        let request: ReadRequest<bytes> = ReadRequest.Page(cursor, Some(page_size));
        let response: ReadResult<bytes, bytes> = perform std.effects.Memory.read(self, request);
        return match response {
            ReadResult.Page(page) => page,
            _ => std.core.abort("scan result ABI invariant"),
        };
    }

    public flow keys(self) -> List<K> ![Error<StorageError>] {
        let store = self.codec_view();
        let context = StorageContext.current();
        var keys: List<K> = [];
        var count: usize = 0;
        var cursor: Option<MemoryCursor> = None;
        while true {
            let page = store.raw_page(cursor, u32 { bits= context.max_page_entries.bits });
            for entry in page.entries {
                if count == context.max_scan_rows { perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "memory key scan exceeds its entry bound", operation= None }); }
                keys = store.decode_key(entry.key) :: keys;
                count = count + 1;
            }
            cursor = page.cursor;
            if cursor.is_none() { return keys.reverse(); }
        }
        return keys.reverse();
    }

    public flow clear(self) -> unit ![Error<StorageError>] {
        let store = self.codec_view();
        let context = StorageContext.current();
        var snapshot: List<(K, MemoryVersion)> = [];
        var count: usize = 0;
        var cursor: Option<MemoryCursor> = None;
        while true {
            let page = store.raw_page(cursor, u32 { bits= context.max_page_entries.bits });
            for entry in page.entries {
                if count == context.max_scan_rows { perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "memory clear snapshot exceeds its entry bound", operation= None }); }
                snapshot = (store.decode_key(entry.key), entry.version) :: snapshot;
                count = count + 1;
            }
            cursor = page.cursor;
            if cursor.is_none() { break; }
        }
        for (key, version) in snapshot.reverse() {
            let intent = store.prepare_delete(key, WriteCondition.Match(version));
            intent.commit_or_raise(false);
        }
    }

    public flow put(self, key: K, value: V) -> unit ![Error<StorageError>] {
        let intent = self.prepare_put(key, value, WriteCondition.Any);
        intent.commit_or_raise(false);
    }

    public flow upsert(self, key: K, value: V) -> unit ![Error<StorageError>] {
        let intent = self.prepare_put(key, value, WriteCondition.Any);
        intent.commit_or_raise(false);
    }

    public flow insert(self, key: K, value: V) -> unit ![Error<StorageError>] {
        let intent = self.prepare_put(key, value, WriteCondition.Missing);
        intent.commit_or_raise(false);
    }

    public flow update(self, key: K, value: V) -> unit ![Error<StorageError>] {
        let intent = self.prepare_put(key, value, WriteCondition.Exists);
        intent.commit_or_raise(false);
    }

    public flow put_versioned(self, key: K, value: V, version: MemoryVersion) -> unit ![Error<StorageError>] {
        let intent = self.prepare_put(key, value, WriteCondition.Match(version));
        intent.commit_or_raise(false);
    }

    public flow delete(self, key: K) -> unit ![Error<StorageError>] {
        let intent = self.prepare_delete(key, WriteCondition.Any);
        intent.commit_or_raise(true);
    }

    public flow delete_versioned(self, key: K, version: MemoryVersion) -> unit ![Error<StorageError>] {
        let intent = self.prepare_delete(key, WriteCondition.Match(version));
        intent.commit_or_raise(false);
    }

    public flow scan(self) -> MemorySelection<V> ![] {
        let store = self;
        return MemorySelection { maximum= None, content_bounds= [], evaluate= (maximum: Option<usize>) => {
            let context = StorageContext.current();
            var output: Array<V> = [];
            var cursor: Option<MemoryCursor> = None;
            while true {
                let remaining = match maximum {
                    None => context.max_scan_rows - output.length,
                    Some(count) => { if count <= output.length { return output; } count - output.length },
                };
                if remaining == 0 { perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "memory selection exceeds its entry bound", operation= None }); }
                let page_size = if remaining < context.max_page_entries { remaining } else { context.max_page_entries };
                let page = store.page(cursor, u32 { bits= page_size.bits });
                for entry in page.entries { output = output.push(entry.value); }
                cursor = page.cursor;
                if cursor.is_none() { return output; }
            }
            return output;
        } };
    }

    public flow select(self, query: MemoryQuery<V>) -> MemorySelection<V> ![Error<StorageError>] {
        query.validate();
        let store = self;
        return MemorySelection { maximum= None, content_bounds= [], evaluate= (maximum: Option<usize>) => {
            if maximum == Some(0) { return []; }
            let context = StorageContext.current();
            let count = match maximum { None => context.max_scan_rows, Some(value) => if value < context.max_scan_rows { value } else { context.max_scan_rows } };
            let view = store.codec_view();
            let request: ReadRequest<bytes> = ReadRequest.Selection(SelectionRequest.Query(query.language, query.version, query.payload, Some(u32 { bits= count.bits })));
            let response: ReadResult<bytes, bytes> = perform std.effects.Memory.read(view, request);
            let page = match response { ReadResult.Selected(page) => view.decode_page(page), _ => std.core.abort("selection result ABI invariant") };
            var output: Array<V> = [];
            for entry in page.entries { output = output.push(entry.value); }
            return output;
        } };
    }

    public flow query(self, query: MemoryQuery<V>) -> MemorySelection<V> ![Error<StorageError>] {
        query.validate();
        let selection = self.select(query);
        return selection;
    }

    public flow related_to(self, query: MemoryQuery<V>) -> MemorySelection<V> ![Error<StorageError>] {
        query.validate();
        let selection = self.select(query);
        return selection;
    }
}

impl MemoryWriteIntent<K ~ StorageKey, V ~ StorageData> {
    // A convenience mutation submits exactly once. In particular, an unknown
    // outcome is recoverable by its reference, never retried as a fresh write.
    flow commit_or_raise(self, absent_delete_is_success: bool) -> unit ![Error<StorageError>] {
        match self.commit() {
            WriteOutcome.Committed(_) => {},
            WriteOutcome.NotCommitted(operation, reason) => match reason {
                MemoryWriteRejection.Unchanged => {
                    if !absent_delete_is_success {
                        perform Error.raise<StorageError>(StorageError {
                            code= "rejected", message= "storage mutation made no change", operation= Some(operation),
                        });
                    }
                },
                MemoryWriteRejection.ConditionConflict(_, _) => {
                    perform Error.raise<StorageError>(StorageError {
                        code= "condition_conflict", message= "storage mutation condition did not match", operation= Some(operation),
                    });
                },
                MemoryWriteRejection.Rejected(error) => {
                    perform Error.raise<StorageError>(StorageError {
                        code= "rejected", message= error.message, operation= Some(operation),
                    });
                },
            },
            WriteOutcome.Unknown(operation) => {
                perform Error.raise<StorageError>(StorageError {
                    code= "outcome_unknown", message= "storage mutation may have committed; reconcile its operation", operation= Some(operation),
                });
            },
        };
    }

    public flow operation_ref(self) -> StorageOperationRef ![] {
        return self.operation;
    }

    public flow commit(self) -> WriteOutcome<MemoryWriteReceipt<K>, MemoryWriteRejection> ![Error<StorageError>] {
        let result: WriteOutcome<MemoryWriteReceipt<bytes>, MemoryWriteRejection> = perform std.effects.Memory.write(self.store, self.operation, self.mutation, self.wire);
        return match result {
            WriteOutcome.Committed(receipt) => WriteOutcome.Committed(self.store.decode_receipt(receipt)),
            WriteOutcome.NotCommitted(reference, reason) => WriteOutcome.NotCommitted(reference, reason),
            WriteOutcome.Unknown(reference) => WriteOutcome.Unknown(reference),
        };
    }
}
