module std.runtime.value;

import std.json.JsonValue;


// A neutral value representation contains no execution/authority identities.
// Capture is the single VM representation operation. Codecs below are source.
public enum Data {
    Unit,
    Bool(bool),
    Int(i128),
    UInt(u128),
    Float(f64),
    String(string),
    Bytes(bytes),
    List(Array<Data>),
    Map(Array<(Data, Data)>),
    Record(Array<(string, Data)>),
    Variant(string, Array<Data>),
    Json(JsonValue),
}

public type DataError = { code: string, message: string };

impl Data {
    // Reflect ordinary checked values through the existing neutral host-value
    // rules. Reject secrets/callables/continuations/capabilities, including
    // nested ones, and enforce the current execution/storage value budget.
    @std.role("std.runtime.value.Data.capture")
    public flow capture<T>(value: T) -> Result<Data, DataError> ![];

    public flow tagged(self, max_bytes: usize) -> Result<string, DataError> ![] {
        // Capture is identity on Data while applying the actual neutral-value
        // node/depth/byte budget to newly source-assembled protocol envelopes.
        let bounded = match Data.capture(self) { Ok(value) => value, Err(error) => { return Err(error); }, };
        var writer = TaggedWriter { parts= [], used= 0, maximum= max_bytes };
        return match writer.value(bounded)? {
            Ok(_) => Ok(writer.finish()),
            Err(error) => Err(error),
        };
    }

    public flow to_json(self) -> Result<JsonValue, DataError> ![] {
        return self.project_json()?;
    }

    flow project_json(self) -> JsonValue ![Error<DataError>] {
        return match self {
            Data.Unit => JsonValue.Null,
            Data.Bool(value) => JsonValue.Bool(value),
            Data.Int(value) => {
                if value < -9223372036854775808 || value > 9223372036854775807 {
                    perform Error.raise<DataError>(DataError { code= "schema_mismatch", message= "signed integer is outside JSON i64 range" });
                }
                JsonValue.Number(std.text.parse_f64(std.text.from_i128(value)).unwrap())
            },
            Data.UInt(value) => {
                if value > 18446744073709551615 {
                    perform Error.raise<DataError>(DataError { code= "schema_mismatch", message= "unsigned integer is outside JSON u64 range" });
                }
                JsonValue.Number(std.text.parse_f64(std.text.from_u128(value)).unwrap())
            },
            Data.Float(value) => {
                if value.bits % 9223372036854775808 >= 9218868437227405312 {
                    perform Error.raise<DataError>(DataError { code= "schema_mismatch", message= "floating-point value is not finite" });
                }
                JsonValue.Number(value)
            },
            Data.String(value) => JsonValue.String(value),
            Data.Bytes(value) => {
                var items: Array<JsonValue> = [];
                for byte in value {
                    items = items.push(JsonValue.Number(std.text.parse_f64(std.text.from_u8(byte)).unwrap()));
                }
                JsonValue.Array(items)
            },
            Data.List(values) => {
                var items: Array<JsonValue> = [];
                for value in values {
                    items = items.push(value.project_json());
                }
                JsonValue.Array(items)
            },
            Data.Map(entries) => {
                var items: Array<JsonValue> = [];
                for (key, value) in entries {
                    items = items.push(JsonValue.Array([key.project_json(), value.project_json()]));
                }
                JsonValue.Array(items)
            },
            Data.Record(fields) => {
                var object: Map<string, JsonValue> = {};
                for (name, value) in fields {
                    if object.contains_key(name) {
                        perform Error.raise<DataError>(DataError { code= "schema_mismatch", message= "record contains duplicate JSON field name" });
                    }
                    object = object.insert(name, value.project_json());
                }
                JsonValue.Object(object)
            },
            Data.Variant(name, fields) => {
                var items: Array<JsonValue> = [];
                for value in fields {
                    items = items.push(value.project_json());
                }
                var object: Map<string, JsonValue> = {};
                object = object.insert("fields", JsonValue.Array(items));
                object = object.insert("name", JsonValue.String(name));
                JsonValue.Object(object)
            },
            Data.Json(value) => value,
        };
    }
}
