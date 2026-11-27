module std.memory;

import std.runtime.value.Data;
import std.crypto.Blake3;


// Readonly identity projection from the actual checked Store handle. It does
// not contain authority, a connection, or a caller-supplied ambient database.
type StoreDescription = { region: string, path: Array<string>, schema_fingerprint: Option<string> };

impl StoreDescription {
    flow validate(self) -> unit ![Error<StorageError>] {
        if self.region == "" { perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid memory write intent", operation= None }); }
        for part in self.path {
            if part == "" { perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid memory write intent", operation= None }); }
        }
        if self.schema_fingerprint == Some("") {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid memory write intent", operation= None });
        }
    }
}

impl StorageContext {
    public flow operation_key(self) -> string ![Error<StorageError>] {
        let expires = self.now_seconds.bits + self.receipt_retention_seconds.bits;
        if self.receipt_retention_seconds == 0 || expires > 9223372036854775807 {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid or expired storage operation key", operation= None });
        }
        let nonce = match StorageContext.nonce() {
            Ok(value) => value,
            Err(error) => perform Error.raise<StorageError>(error),
        };
        return "so1:" + expires.hex_fixed(16) + ":" + nonce.hex_fixed(32);
    }
}

impl u128 {
    flow hex_fixed(self, width: usize) -> string ![] {
        var output = "";
        var value = self;
        var index: usize = 0;
        while index < width {
            let digit = match value % 16 {
                0 => "0", 1 => "1", 2 => "2", 3 => "3",
                4 => "4", 5 => "5", 6 => "6", 7 => "7",
                8 => "8", 9 => "9", 10 => "a", 11 => "b",
                12 => "c", 13 => "d", 14 => "e", _ => "f",
            };
            output = digit + output;
            value = value / 16;
            index = index + 1;
        }
        return output;
    }
}

impl MemoryVersion {
    flow checked_token(self) -> string ![Error<StorageError>] {
        let text = self.opaque;
        let bytes = text.utf8;
        if bytes.length != 118 || !std.text.starts_with(text, "mv1:") || bytes.get(68) != Some(58) || bytes.get(101) != Some(58) {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid or obsolete memory version token", operation= None });
        }
        var at: usize = 4;
        var revision: u128 = 0;
        while at < bytes.length {
            if at != 68 && at != 101 {
                let byte = bytes.get(at).unwrap();
                let digit = if byte >= 48 && byte <= 57 { byte.bits - 48 }
                    else if byte >= 97 && byte <= 102 { byte.bits - 87 }
                    else { perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid or obsolete memory version token", operation= None }); };
                if at >= 102 { revision = revision * 16 + digit; }
            }
            at = at + 1;
        }
        if revision == 0 || revision > 9223372036854775807 {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid or obsolete memory version token", operation= None });
        }
        return text;
    }
}

impl WriteCondition {
    flow token(self) -> string ![Error<StorageError>] {
        return match self {
            WriteCondition.Any => "any",
            WriteCondition.Missing => "missing",
            WriteCondition.Exists => "exists",
            WriteCondition.Match(version) => version.checked_token(),
        };
    }

    flow data(self) -> Data ![] {
        return match self {
            WriteCondition.Any => Data.Variant("Any", []),
            WriteCondition.Missing => Data.Variant("Missing", []),
            WriteCondition.Exists => Data.Variant("Exists", []),
            WriteCondition.Match(version) => Data.Variant("Match", [Data.String(version.opaque)]),
        };
    }
}

type MutationFingerprint = { private input: bytes, private used: usize, private maximum: usize };

impl MutationFingerprint {
    flow little_endian(value: u128) -> bytes ![] {
        var remaining = value;
        var output: Array<u8> = [];
        var index: usize = 0;
        while index < 8 {
            output = output.push(u8 { bits= remaining % 256 });
            remaining = remaining / 256;
            index = index + 1;
        }
        return bytes { data= output };
    }

    flow new(store: StoreDescription, kind: string, maximum: usize) -> MutationFingerprint ![Error<StorageError>] {
        let presence: u8 = if store.schema_fingerprint.is_some() { 1 } else { 0 };
        var output = MutationFingerprint {
            input= "etas.memory.mutation.v1".utf8 + bytes { data= [presence] } + MutationFingerprint.little_endian(store.path.length.bits),
            used= 0,
            maximum= maximum,
        };
        output.part(store.region);
        match store.schema_fingerprint {
            Some(schema) => output.part(schema),
            None => {},
        };
        for part in store.path { output.part(part); }
        output.part(kind);
        return output;
    }

    flow part(var self, text: string) -> unit ![Error<StorageError>] {
        if text.length > self.maximum - self.used {
            perform Error.raise<StorageError>(StorageError { code= "limit_exceeded", message= "memory mutation or receipt storage limit exceeded", operation= None });
        }
        self.used = self.used + text.length;
        self.input = self.input + MutationFingerprint.little_endian(text.length.bits) + text.utf8;
    }

    flow value(var self, value: Data) -> unit ![Error<StorageError>] {
        let text = match value.tagged(self.maximum) {
            Ok(value) => value,
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message, operation= None }),
        };
        self.part(text);
    }

    @std.role("std.memory.MutationFingerprint.cached_schema")
    flow cached_schema(input: bytes) -> Option<string> ![];

    @std.role("std.memory.MutationFingerprint.remember_schema")
    flow remember_schema(input: bytes, digest: string) -> unit ![];

    flow finish_schema(self) -> string ![] {
        return match MutationFingerprint.cached_schema(self.input) {
            Some(digest) => digest,
            None => {
                let digest = self.finish();
                MutationFingerprint.remember_schema(self.input, digest);
                digest
            },
        };
    }

    flow finish(self) -> string ![] {
        let digest = Blake3.hash(self.input);
        var output = "";
        for byte in digest { output = output + byte.bits.hex_fixed(2); }
        return output;
    }
}

type IntentEnvelope = {};

impl IntentEnvelope {
    flow encode(store: StoreDescription, mutation: Data, operation: StorageOperationRef, maximum: usize) -> string ![Error<StorageError>] {
        let schema = match store.schema_fingerprint { None => Data.Unit, Some(value) => Data.String(value) };
        var path: Array<Data> = [];
        for part in store.path { path = path.push(Data.String(part)); }
        let envelope = Data.Record([
            ("format", Data.String("etas.memory.write-intent.v1")),
            ("region", Data.String(store.region)),
            ("schema", schema),
            ("path", Data.List(path)),
            ("operation", Data.String(operation.key)),
            ("fingerprint", Data.String(operation.fingerprint)),
            ("mutation", mutation),
        ]);
        return match envelope.tagged(maximum) {
            Ok(text) => text,
            Err(error) => perform Error.raise<StorageError>(StorageError { code= error.code, message= error.message, operation= None }),
        };
    }
}

impl StorageOperationRef {
    public flow validate(self) -> unit ![Error<StorageError>] {
        let token = self.key.utf8;
        if token.length != 53 || !std.text.starts_with(self.key, "so1:") || token.get(20) != Some(58) {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid or expired storage operation key", operation= None });
        }
        var expires: u128 = 0;
        var at: usize = 4;
        while at < token.length {
            if at != 20 {
                let byte = token.get(at).unwrap();
                let digit = if byte >= 48 && byte <= 57 { byte.bits - 48 }
                    else if byte >= 97 && byte <= 102 { byte.bits - 87 }
                    else { perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid or expired storage operation key", operation= None }); };
                if at < 20 { expires = expires * 16 + digit; }
            }
            at = at + 1;
        }
        if expires == 0 || expires > 9223372036854775807 {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid or expired storage operation key", operation= None });
        }
        if self.fingerprint.length != 64 {
            perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid storage request fingerprint", operation= None });
        }
        for byte in self.fingerprint.utf8 {
            if !((byte >= 48 && byte <= 57) || (byte >= 97 && byte <= 102)) {
                perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid storage request fingerprint", operation= None });
            }
        }
        // Expiry is deliberately not a local claim of non-commit. Reconcile
        // asks the authorized backend and may return Expired or Unresolved.
    }
}
