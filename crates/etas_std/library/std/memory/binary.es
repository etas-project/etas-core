module std.memory;

// Lossless data representation at the storage boundary. These are private
// data, never authority-bearing runtime values. Restoring a nominal value or
// trust provenance still requires the checked typed runtime boundary.
enum StorageValue {
    Unit,
    Bool(bool),
    Scalar(u8, u8, u128),
    String(string),
    Bytes(bytes),
    Sequence(u8, Array<StorageValue>),
    Map(Array<(StorageValue, StorageValue)>),
    Record(Array<(string, StorageValue)>),
    Nominal(string, StorageValue),
    Variant(string, string, Array<StorageValue>),
    Trust(u8, bytes, StorageValue),
}

impl StorageValue {
    @std.role("std.memory.StorageValue.capture")
    flow capture<T>(value: T) -> Result<StorageValue, StorageError> ![];

    @std.role("std.memory.StorageValue.restore")
    flow restore<T>(value: StorageValue) -> Result<T, StorageError> ![];

    @std.role("std.memory.StorageValue.schema")
    flow schema<T>() -> string ![];
}

type StorageBinary = {
    private maximum_bytes: usize,
    private maximum_nodes: usize,
    private maximum_depth: usize,
};

type StorageMeasure = { private bytes_left: usize, private nodes_left: usize, private maximum_depth: usize };
type StorageWriter = { private output: std.bytes.BytesBuilder, private key: bool };
type StorageReader = {
    private check_encoding: bool,
    private key: bool,
    private input: bytes,
    private offset: usize,
    private nodes_left: usize,
    private maximum_depth: usize,
};

impl StorageBinary {
    flow failure(code: string, message: string) -> StorageError ![] {
        return StorageError { code= code, message= message, operation= None };
    }

    flow scalar_width(tag: u8, width: u8) -> Option<usize> ![] {
        let fixed: usize = match tag {
            16 => 1, 17 => 2, 18 => 4, 19 => 8, 20 => 16,
            24 => 1, 25 => 2, 26 => 4, 27 => 8, 28 => 16,
            32 => 4, 33 => 8, 34 => 4,
            _ => 0,
        };
        if fixed != 0 && width.bits == fixed.bits { return Some(fixed); }
        if (tag == 21 || tag == 29) && (width == 4 || width == 8) {
            return Some(usize { bits= width.bits });
        }
        return None;
    }

    flow compare_bytes(left: bytes, right: bytes) -> i32 ![] {
        var index: usize = 0;
        while index < left.length && index < right.length {
            let a = left.get(index).unwrap();
            let b = right.get(index).unwrap();
            if a < b { return -1; }
            if a > b { return 1; }
            index = index + 1;
        }
        if left.length < right.length { return -1; }
        if left.length > right.length { return 1; }
        return 0;
    }

    flow encode(self, schema: string, value: StorageValue, key: bool) -> Result<bytes, StorageError> ![] {
        var measure = StorageMeasure {
            bytes_left= self.maximum_bytes, nodes_left= self.maximum_nodes,
            maximum_depth= self.maximum_depth,
        };
        match measure.reserve(5) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        match measure.text(schema) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        match measure.value(value, key, 1) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        var writer = StorageWriter { output= std.bytes.BytesBuilder.new(), key= key };
        writer.append(bytes { data= [69, 84, 83, 1, if key { 1 } else { 0 }] });
        writer.blob(schema.utf8);
        match writer.value(value) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        return Ok(writer.output.build());
    }

    flow decode(self, schema: string, input: bytes, key: bool) -> Result<StorageValue, StorageError> ![] {
        if input.length > self.maximum_bytes {
            return Err(StorageBinary.failure("limit_exceeded", "storage input exceeds its byte budget"));
        }
        var reader = StorageReader {
            key= key, check_encoding= false,
            input= input, offset= 0, nodes_left= self.maximum_nodes,
            maximum_depth= self.maximum_depth,
        };
        let expected: Array<u8> = [69, 84, 83, 1, if key { 1 } else { 0 }];
        for byte in expected {
            let actual = match reader.byte() { Ok(value) => value, Err(error) => { return Err(error); }, };
            if actual != byte {
                return Err(StorageBinary.failure("schema_mismatch", "storage format, version or codec mode differs"));
            }
        }
        let actual_schema = match reader.blob() { Ok(value) => value, Err(error) => { return Err(error); }, };
        if actual_schema != schema.utf8 {
            return match std.codec.text.utf8_decode(actual_schema, std.codec.text.MalformedInput.Strict) {
                Ok(_) => Err(StorageBinary.failure("schema_mismatch", "storage type schema differs")),
                Err(_) => Err(StorageBinary.failure("invalid_encoding", "storage text is not valid UTF-8")),
            };
        }
        let value = match reader.value(1) { Ok(value) => value, Err(error) => { return Err(error); }, };
        if reader.offset != input.length {
            return Err(StorageBinary.failure("invalid_encoding", "storage value has trailing bytes"));
        }
        // Fixed-width scalars, valid UTF-8 and strictly ordered record fields
        // already determine a unique encoding. Sets and maps also need the
        // writer's canonical-key ordering and duplicate checks.
        if reader.check_encoding {
            let encoded = match self.encode(schema, value, key) { Ok(value) => value, Err(error) => { return Err(error); }, };
            if encoded != input {
                return Err(StorageBinary.failure("invalid_encoding", "storage value is not in canonical layout"));
            }
        }
        return Ok(value);
    }
}

impl StorageMeasure {
    flow reserve(var self, count: usize) -> Result<unit, StorageError> ![] {
        if count > self.bytes_left {
            return Err(StorageBinary.failure("limit_exceeded", "storage value exceeds its byte budget"));
        }
        self.bytes_left = self.bytes_left - count;
        return Ok(());
    }

    flow blob(var self, value: bytes) -> Result<unit, StorageError> ![] {
        match self.reserve(8) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        return self.reserve(value.length);
    }

    flow text(var self, value: string) -> Result<unit, StorageError> ![] {
        match self.reserve(8) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        return self.reserve(value.length);
    }

    flow values(var self, values: Array<StorageValue>, key: bool, depth: usize) -> Result<unit, StorageError> ![] {
        match self.reserve(8) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        for value in values {
            match self.value(value, key, depth) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        }
        return Ok(());
    }

    flow value(var self, value: StorageValue, key: bool, depth: usize) -> Result<unit, StorageError> ![] {
        if depth > self.maximum_depth || self.nodes_left == 0 {
            return Err(StorageBinary.failure("limit_exceeded", "storage value exceeds its node or depth budget"));
        }
        self.nodes_left = self.nodes_left - 1;
        match self.reserve(1) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        return match value {
            StorageValue.Unit => Ok(()),
            StorageValue.Bool(_) => self.reserve(1),
            StorageValue.Scalar(tag, width, bits) => {
                let count = match StorageBinary.scalar_width(tag, width) {
                    Some(count) => count,
                    None => { return Err(StorageBinary.failure("invalid_encoding", "unknown storage scalar or width")); },
                };
                if key && (tag == 32 || tag == 33) {
                    return Err(StorageBinary.failure("schema_mismatch", "raw floats are not storage keys"));
                }
                var remaining = bits;
                var index: usize = 0;
                while index < count { remaining = remaining / 256; index = index + 1; }
                if remaining != 0 || (tag == 34 && (bits > 1114111 || (bits >= 55296 && bits <= 57343))) {
                    return Err(StorageBinary.failure("invalid_encoding", "storage scalar is outside its representation"));
                }
                self.reserve(count + if tag == 21 || tag == 29 { 1 } else { 0 })
            },
            StorageValue.String(value) => self.text(value),
            StorageValue.Bytes(value) => self.blob(value),
            StorageValue.Sequence(kind, values) => {
                if kind < 64 || kind > 71 {
                    return Err(StorageBinary.failure("invalid_encoding", "unknown storage sequence kind"));
                }
                self.values(values, key || kind == 71, depth + 1)
            },
            StorageValue.Map(entries) => {
                match self.reserve(8) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                for (entry_key, entry_value) in entries {
                    match self.value(entry_key, true, depth + 1) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                    match self.value(entry_value, key, depth + 1) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                }
                Ok(())
            },
            StorageValue.Record(fields) => {
                match self.reserve(8) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                for (name, field) in fields {
                    match self.text(name) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                    match self.value(field, key, depth + 1) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                }
                Ok(())
            },
            StorageValue.Nominal(identity, inner) => {
                match self.text(identity) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                self.value(inner, key, depth + 1)
            },
            StorageValue.Variant(identity, name, fields) => {
                match self.text(identity) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                match self.text(name) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                self.values(fields, key, depth + 1)
            },
            StorageValue.Trust(label, provenance, inner) => {
                if label > 3 {
                    return Err(StorageBinary.failure("invalid_encoding", "unknown storage trust label"));
                }
                match self.reserve(1) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                match self.blob(provenance) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                self.value(inner, key, depth + 1)
            },
        };
    }
}

impl StorageWriter {
    flow byte(var self, value: u8) -> unit ![] {
        var output = self.output;
        output.push(value);
        self.output = output;
    }

    flow append(var self, value: bytes) -> unit ![] {
        var output = self.output;
        output.append(value);
        self.output = output;
    }

    flow integer(var self, value: u128, width: usize) -> unit ![] {
        var remaining = value;
        var index: usize = 0;
        while index < width {
            self.byte(u8 { bits= remaining % 256 });
            remaining = remaining / 256;
            index = index + 1;
        }
    }

    flow blob(var self, value: bytes) -> unit ![] {
        self.integer(value.length.bits, 8);
        self.append(value);
    }

    flow fragment(value: StorageValue, key: bool) -> Result<bytes, StorageError> ![] {
        var writer = StorageWriter { output= std.bytes.BytesBuilder.new(), key= key };
        match writer.value(value) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        return Ok(writer.output.build());
    }

    flow values(var self, values: Array<StorageValue>) -> Result<unit, StorageError> ![] {
        self.integer(values.length.bits, 8);
        for value in values {
            match self.value(value) { Ok(_) => {}, Err(error) => { return Err(error); }, };
        }
        return Ok(());
    }

    flow value(var self, value: StorageValue) -> Result<unit, StorageError> ![] {
        match value {
            StorageValue.Unit => { self.byte(0); },
            StorageValue.Bool(value) => { self.byte(1); self.byte(if value { 1 } else { 0 }); },
            StorageValue.Scalar(tag, width, bits) => {
                self.byte(tag);
                if tag == 21 || tag == 29 { self.byte(width); }
                self.integer(bits, usize { bits= width.bits });
            },
            StorageValue.String(value) => { self.byte(48); self.blob(value.utf8); },
            StorageValue.Bytes(value) => { self.byte(49); self.blob(value); },
            StorageValue.Sequence(kind, values) => {
                self.byte(kind);
                if kind != 71 {
                    return self.values(values);
                }
                var keyed: Array<(bytes, StorageValue)> = [];
                for item in values {
                    let key = match StorageWriter.fragment(item, true) { Ok(value) => value, Err(error) => { return Err(error); }, };
                    keyed = keyed.push((key, item));
                }
                let sorted = std.collections.sort_by(keyed, (a: (bytes, StorageValue), b: (bytes, StorageValue)) => {
                    let (left, _) = a; let (right, _) = b;
                    return StorageBinary.compare_bytes(left, right);
                });
                var previous: Option<bytes> = None;
                for (encoded, _) in sorted {
                    match previous {
                        Some(old) => { if old == encoded { return Err(StorageBinary.failure("invalid_encoding", "duplicate storage set element")); } },
                        None => {},
                    };
                    previous = Some(encoded);
                }
                self.integer(values.length.bits, 8);
                if self.key {
                    for (encoded, _) in sorted { self.append(encoded); }
                } else {
                    for item in values { match self.value(item) { Ok(_) => {}, Err(error) => { return Err(error); }, }; }
                }
            },
            StorageValue.Map(entries) => {
                self.byte(72);
                var keyed: Array<(bytes, StorageValue, StorageValue)> = [];
                for (entry_key, entry_value) in entries {
                    let encoded = match StorageWriter.fragment(entry_key, true) { Ok(value) => value, Err(error) => { return Err(error); }, };
                    keyed = keyed.push((encoded, entry_key, entry_value));
                }
                let sorted = std.collections.sort_by(keyed, (a: (bytes, StorageValue, StorageValue), b: (bytes, StorageValue, StorageValue)) => {
                    let (left, _, _) = a; let (right, _, _) = b;
                    return StorageBinary.compare_bytes(left, right);
                });
                var previous: Option<bytes> = None;
                for (encoded, _, _) in sorted {
                    match previous {
                        Some(old) => { if old == encoded { return Err(StorageBinary.failure("invalid_encoding", "duplicate storage map key")); } },
                        None => {},
                    };
                    previous = Some(encoded);
                }
                self.integer(entries.length.bits, 8);
                if self.key {
                    for (encoded, _, entry_value) in sorted {
                        self.append(encoded);
                        match self.value(entry_value) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                    }
                } else {
                    for (entry_key, entry_value) in entries {
                        match self.value(entry_key) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                        match self.value(entry_value) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                    }
                }
            },
            StorageValue.Record(fields) => {
                self.byte(80);
                let sorted = std.collections.sort_by(fields, (a: (string, StorageValue), b: (string, StorageValue)) => {
                    let (left, _) = a; let (right, _) = b;
                    return if left < right { -1 } else if left > right { 1 } else { 0 };
                });
                self.integer(sorted.length.bits, 8);
                var previous: Option<string> = None;
                for (name, field) in sorted {
                    match previous {
                        Some(old) => { if old == name { return Err(StorageBinary.failure("invalid_encoding", "duplicate storage record field")); } },
                        None => {},
                    };
                    previous = Some(name);
                    self.blob(name.utf8);
                    match self.value(field) { Ok(_) => {}, Err(error) => { return Err(error); }, };
                }
            },
            StorageValue.Nominal(identity, inner) => {
                self.byte(81); self.blob(identity.utf8);
                match self.value(inner) { Ok(_) => {}, Err(error) => { return Err(error); }, };
            },
            StorageValue.Variant(identity, name, fields) => {
                self.byte(82); self.blob(identity.utf8); self.blob(name.utf8);
                return self.values(fields);
            },
            StorageValue.Trust(label, provenance, inner) => {
                self.byte(83); self.byte(label); self.blob(provenance);
                match self.value(inner) { Ok(_) => {}, Err(error) => { return Err(error); }, };
            },
        };
        return Ok(());
    }
}

impl StorageReader {
    flow byte(var self) -> Result<u8, StorageError> ![] {
        if self.offset == self.input.length {
            return Err(StorageBinary.failure("invalid_encoding", "truncated storage value"));
        }
        let value = self.input.get(self.offset).unwrap();
        self.offset = self.offset + 1;
        return Ok(value);
    }

    flow integer(var self, width: usize) -> Result<u128, StorageError> ![] {
        if width > self.input.length - self.offset {
            return Err(StorageBinary.failure("invalid_encoding", "truncated storage scalar"));
        }
        var value: u128 = 0;
        var place: u128 = 1;
        var index: usize = 0;
        while index < width {
            value = value + self.input.get(self.offset).unwrap().bits * place;
            self.offset = self.offset + 1;
            index = index + 1;
            if index < width { place = place * 256; }
        }
        return Ok(value);
    }

    flow count(var self, maximum: usize) -> Result<usize, StorageError> ![] {
        let value = match self.integer(8) { Ok(value) => value, Err(error) => { return Err(error); }, };
        if value > maximum.bits {
            return Err(StorageBinary.failure("invalid_encoding", "storage length exceeds its available input or node budget"));
        }
        return Ok(usize { bits= value });
    }

    flow blob(var self) -> Result<bytes, StorageError> ![] {
        let length = match self.count(self.input.length - self.offset) { Ok(value) => value, Err(error) => { return Err(error); }, };
        if length > self.input.length - self.offset {
            return Err(StorageBinary.failure("invalid_encoding", "truncated storage byte string"));
        }
        let start = self.offset;
        self.offset = start + length;
        return Ok(self.input[start, self.offset)?.unwrap());
    }

    flow text(var self) -> Result<string, StorageError> ![] {
        let value = match self.blob() { Ok(value) => value, Err(error) => { return Err(error); }, };
        return match std.codec.text.utf8_decode(value, std.codec.text.MalformedInput.Strict) {
            Ok(value) => Ok(value),
            Err(_) => Err(StorageBinary.failure("invalid_encoding", "storage text is not valid UTF-8")),
        };
    }

    flow values(var self, depth: usize) -> Result<Array<StorageValue>, StorageError> ![] {
        let count = match self.count(self.nodes_left) { Ok(value) => value, Err(error) => { return Err(error); }, };
        var output: Array<StorageValue> = [];
        var index: usize = 0;
        while index < count {
            let value = match self.value(depth) { Ok(value) => value, Err(error) => { return Err(error); }, };
            output = output.push(value);
            index = index + 1;
        }
        return Ok(output);
    }

    flow value(var self, depth: usize) -> Result<StorageValue, StorageError> ![] {
        if depth > self.maximum_depth || self.nodes_left == 0 {
            return Err(StorageBinary.failure("limit_exceeded", "storage value exceeds its node or depth budget"));
        }
        self.nodes_left = self.nodes_left - 1;
        let tag = match self.byte() { Ok(value) => value, Err(error) => { return Err(error); }, };
        if tag == 0 { return Ok(StorageValue.Unit); }
        if tag == 1 {
            let value = match self.byte() { Ok(value) => value, Err(error) => { return Err(error); }, };
            if value > 1 { return Err(StorageBinary.failure("invalid_encoding", "invalid storage boolean")); }
            return Ok(StorageValue.Bool(value == 1));
        }
        if (tag >= 16 && tag <= 21) || (tag >= 24 && tag <= 29) || (tag >= 32 && tag <= 34) {
            let width: u8 = if tag == 21 || tag == 29 {
                match self.byte() { Ok(value) => value, Err(error) => { return Err(error); }, }
            } else {
                match tag { 16 => 1, 17 => 2, 18 => 4, 19 => 8, 20 => 16, 24 => 1, 25 => 2, 26 => 4, 27 => 8, 28 => 16, 32 => 4, 33 => 8, _ => 4, }
            };
            let count = match StorageBinary.scalar_width(tag, width) {
                Some(value) => value,
                None => { return Err(StorageBinary.failure("invalid_encoding", "invalid storage scalar width")); },
            };
            let bits = match self.integer(count) { Ok(value) => value, Err(error) => { return Err(error); }, };
            if (self.key && (tag == 32 || tag == 33)) ||
                (tag == 34 && (bits > 1114111 || (bits >= 55296 && bits <= 57343))) {
                self.check_encoding = true;
            }
            return Ok(StorageValue.Scalar(tag, width, bits));
        }
        if tag == 48 {
            let text = match self.text() { Ok(value) => value, Err(error) => { return Err(error); }, };
            return Ok(StorageValue.String(text));
        }
        if tag == 49 {
            let data = match self.blob() { Ok(value) => value, Err(error) => { return Err(error); }, };
            return Ok(StorageValue.Bytes(data));
        }
        if tag >= 64 && tag <= 71 {
            if tag == 71 { self.check_encoding = true; }
            let values = match self.values(depth + 1) { Ok(value) => value, Err(error) => { return Err(error); }, };
            return Ok(StorageValue.Sequence(tag, values));
        }
        if tag == 72 {
            self.check_encoding = true;
            let count = match self.count(self.nodes_left / 2) { Ok(value) => value, Err(error) => { return Err(error); }, };
            var entries: Array<(StorageValue, StorageValue)> = [];
            var index: usize = 0;
            while index < count {
                let key = match self.value(depth + 1) { Ok(value) => value, Err(error) => { return Err(error); }, };
                let value = match self.value(depth + 1) { Ok(value) => value, Err(error) => { return Err(error); }, };
                entries = entries.push((key, value));
                index = index + 1;
            }
            return Ok(StorageValue.Map(entries));
        }
        if tag == 80 {
            let count = match self.count(self.nodes_left) { Ok(value) => value, Err(error) => { return Err(error); }, };
            var fields: Array<(string, StorageValue)> = [];
            var previous = "";
            var index: usize = 0;
            while index < count {
                let name = match self.text() { Ok(value) => value, Err(error) => { return Err(error); }, };
                let value = match self.value(depth + 1) { Ok(value) => value, Err(error) => { return Err(error); }, };
                if index > 0 && name <= previous { self.check_encoding = true; }
                previous = name;
                fields = fields.push((name, value));
                index = index + 1;
            }
            return Ok(StorageValue.Record(fields));
        }
        if tag == 81 {
            let identity = match self.text() { Ok(value) => value, Err(error) => { return Err(error); }, };
            let value = match self.value(depth + 1) { Ok(value) => value, Err(error) => { return Err(error); }, };
            return Ok(StorageValue.Nominal(identity, value));
        }
        if tag == 82 {
            let identity = match self.text() { Ok(value) => value, Err(error) => { return Err(error); }, };
            let name = match self.text() { Ok(value) => value, Err(error) => { return Err(error); }, };
            let fields = match self.values(depth + 1) { Ok(value) => value, Err(error) => { return Err(error); }, };
            return Ok(StorageValue.Variant(identity, name, fields));
        }
        if tag == 83 {
            let label = match self.byte() { Ok(value) => value, Err(error) => { return Err(error); }, };
            let provenance = match self.blob() { Ok(value) => value, Err(error) => { return Err(error); }, };
            let inner = match self.value(depth + 1) { Ok(value) => value, Err(error) => { return Err(error); }, };
            if label > 3 { self.check_encoding = true; }
            return Ok(StorageValue.Trust(label, provenance, inner));
        }
        return Err(StorageBinary.failure("invalid_encoding", "unknown storage value tag"));
    }
}
