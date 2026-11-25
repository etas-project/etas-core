module std.memory;

// Admission is structural checked evidence. A codec cannot grant admission
// to an execution capability or remove a nested trust/secrecy restriction.
public spec StorageData {}
public spec StorageKey ~ StorageData + std.collections.Eq {}

public type StorageCodec<T> = {
    format: string,
    version: u32,
    encode: (T) -> Result<bytes, StorageError> ![],
    decode: (bytes) -> Result<T, StorageError> ![],
};

public type MemoryQuery<V> = {
    language: string,
    version: u32,
    payload: bytes,
};

public flow codec<T ~ StorageData>() -> StorageCodec<T> ![] {
    let schema = StorageValue.schema<T>();
    let profile = StorageBinary { maximum_bytes= 1048576, maximum_nodes= 16384, maximum_depth= 48 };
    return StorageCodec {
        format= "etas.storage.binary",
        version= 1,
        encode= (value: T) => {
            let data = match StorageValue.capture<T>(value) {
                Ok(data) => data,
                Err(error) => { return Err(error); },
            };
            return profile.encode(schema, data, false);
        },
        decode= (input: bytes) => {
            let data = match profile.decode(schema, input, false) {
                Ok(data) => data,
                Err(error) => { return Err(error); },
            };
            return StorageValue.restore<T>(data);
        },
    };
}

public flow key_codec<K ~ StorageKey>() -> StorageCodec<K> ![] {
    let schema = StorageValue.schema<K>();
    let profile = StorageBinary { maximum_bytes= 1048576, maximum_nodes= 16384, maximum_depth= 48 };
    return StorageCodec {
        format= "etas.storage.binary.key",
        version= 1,
        encode= (value: K) => {
            let data = match StorageValue.capture<K>(value) {
                Ok(data) => data,
                Err(error) => { return Err(error); },
            };
            return profile.encode(schema, data, true);
        },
        decode= (input: bytes) => {
            let data = match profile.decode(schema, input, true) {
                Ok(data) => data,
                Err(error) => { return Err(error); },
            };
            return StorageValue.restore<K>(data);
        },
    };
}

public flow query<Q ~ StorageData, V>(language: string, version: u32, value: Q, codec: StorageCodec<Q> = std.memory.codec<Q>()) -> MemoryQuery<V> ![Error<StorageError>] {
    if language == "" || version == 0 {
        perform Error.raise<StorageError>(StorageError {
            code= "invalid_request",
            message= "memory query requires a nonempty language and positive version",
            operation= None,
        });
    }
    codec.validate();
    let payload = match (codec.encode)(value) {
        Ok(payload) => payload,
        Err(error) => perform Error.raise<StorageError>(error),
    };
    return MemoryQuery { language= language, version= version, payload= payload };
}

impl StorageCodec<T> {
    flow validate(self) -> unit ![Error<StorageError>] {
        if self.format == "" || self.version == 0 {
            perform Error.raise<StorageError>(StorageError {
                code= "invalid_request",
                message= "storage codec requires a nonempty format and positive version",
                operation= None,
            });
        }
    }
}

impl MemoryQuery<V> {
    flow validate(self) -> unit ![Error<StorageError>] {
        if self.language == "" || self.version == 0 {
            perform Error.raise<StorageError>(StorageError {
                code= "invalid_request",
                message= "memory query requires a nonempty language and positive version",
                operation= None,
            });
        }
    }
}
