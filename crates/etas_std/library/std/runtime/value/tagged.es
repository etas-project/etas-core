module std.runtime.value;

import std.json.JsonValue;


type TaggedWriter = { private output: string, private maximum: usize };

impl TaggedWriter {
    flow raw(var self, text: string) -> unit ![Error<DataError>] {
        if text.length > self.maximum - self.output.length {
            perform Error.raise<DataError>(DataError { code= "budget_exceeded", message= "stored value exceeds codec limits" });
        }
        self.output = self.output + text;
    }

    flow quote(var self, value: string) -> unit ![Error<DataError>] {
        self.raw("\"");
        for scalar in std.text.chars(value) {
            self.raw(match scalar.bits {
                34 => "\\\"",
                92 => "\\\\",
                8 => "\\b",
                12 => "\\f",
                10 => "\\n",
                13 => "\\r",
                9 => "\\t",
                _ => {
                    let code = scalar.to_u32();
                    if code < 32 {
                        let high = std.text.char_get("0123456789abcdef", code / 16).unwrap();
                        let low = std.text.char_get("0123456789abcdef", code % 16).unwrap();
                        "\\u00" + std.text.from_char(high) + std.text.from_char(low)
                    } else { std.text.from_char(scalar) }
                },
            });
        }
        self.raw("\"");
    }

    flow number(var self, value: f64) -> unit ![Error<DataError>] {
        match std.json.stringify(JsonValue.Number(value)) {
            Ok(text) => self.raw(text),
            Err(_) => perform Error.raise<DataError>(DataError { code= "schema_mismatch", message= "non-finite host float cannot be stored" }),
        };
    }

    flow scalar_header(var self, kind: string) -> unit ![Error<DataError>] {
        self.raw("{\"kind\":");
        self.quote(kind);
        self.raw(",\"value\":");
    }

    flow sequence(var self, values: Array<Data>) -> unit ![Error<DataError>] {
        self.raw("[");
        var separator = "";
        for value in values {
            self.raw(separator);
            self.value(value);
            separator = ",";
        }
        self.raw("]");
    }

    flow value(var self, value: Data) -> unit ![Error<DataError>] {
        match value {
            Data.Unit => self.raw("{\"kind\":\"unit\"}"),
            Data.Bool(value) => {
                self.scalar_header("bool");
                self.raw(if value { "true" } else { "false" });
                self.raw("}");
            },
            Data.Int(value) => {
                self.scalar_header("int"); self.quote(std.text.from_i128(value)); self.raw("}");
            },
            Data.UInt(value) => {
                self.scalar_header("uint"); self.quote(std.text.from_u128(value)); self.raw("}");
            },
            Data.Float(value) => {
                self.scalar_header("float"); self.number(value); self.raw("}");
            },
            Data.String(value) => {
                self.scalar_header("string"); self.quote(value); self.raw("}");
            },
            Data.Bytes(value) => {
                self.scalar_header("bytes"); self.raw("[");
                var index: usize = 0;
                while index < value.length {
                    let digits = std.text.from_u8(value[index]?.unwrap());
                    self.raw(if index == 0 { digits } else { "," + digits });
                    index = index + 1;
                }
                self.raw("]}");
            },
            Data.List(values) => {
                self.raw("{\"items\":"); self.sequence(values); self.raw(",\"kind\":\"list\"}");
            },
            Data.Map(entries) => {
                self.raw("{\"entries\":[");
                var separator = "";
                for (key, value) in entries {
                    self.raw(separator); self.raw("{\"key\":"); self.value(key);
                    self.raw(",\"value\":"); self.value(value); self.raw("}"); separator = ",";
                }
                self.raw("],\"kind\":\"map\"}");
            },
            Data.Record(fields) => {
                self.raw("{\"fields\":[");
                var seen: std.collections.OrderedSet<string> = std.collections.OrderedSet.new();
                var separator = "";
                for (name, value) in fields {
                    if seen.contains(name) {
                        perform Error.raise<DataError>(DataError { code= "schema_mismatch", message= "duplicate stored record field" });
                    }
                    seen = seen.insert(name);
                    self.raw(separator); self.raw("{\"name\":"); self.quote(name);
                    self.raw(",\"value\":"); self.value(value); self.raw("}"); separator = ",";
                }
                self.raw("],\"kind\":\"record\"}");
            },
            Data.Variant(name, fields) => {
                self.raw("{\"fields\":"); self.sequence(fields);
                self.raw(",\"kind\":\"variant\",\"name\":"); self.quote(name); self.raw("}");
            },
            Data.Json(value) => {
                self.scalar_header("json"); self.json(value); self.raw("}");
            },
        };
    }

    flow json(var self, value: JsonValue) -> unit ![Error<DataError>] {
        match value {
            JsonValue.Null => self.raw("null"),
            JsonValue.Bool(value) => self.raw(if value { "true" } else { "false" }),
            JsonValue.Number(value) => self.number(value),
            JsonValue.String(value) => self.quote(value),
            JsonValue.Array(values) => {
                self.raw("["); var separator = "";
                for value in values {
                    self.raw(separator); self.json(value); separator = ",";
                }
                self.raw("]");
            },
            JsonValue.Object(fields) => {
                var ordered: std.collections.OrderedMap<string, JsonValue> = std.collections.OrderedMap.new();
                for (name, value) in fields {
                    ordered = ordered.insert(name, value);
                }
                self.raw("{"); var separator = "";
                for (name, value) in ordered.iter() {
                    self.raw(separator); self.quote(name); self.raw(":"); self.json(value); separator = ",";
                }
                self.raw("}");
            },
        };
    }


}
