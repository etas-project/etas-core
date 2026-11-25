module std.json;

type JsonWriter = {};

impl JsonWriter {
    flow quote(self, text: string) -> string ![] {
        var output = "\"";
        for scalar in std.text.chars(text) {
            output = output + match scalar.bits {
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
                    } else {
                        std.text.from_char(scalar)
                    }
                },
            };
        }
        return output + "\"";
    }

    flow number(self, value: f64) -> string ![Error<JsonError>] {
        if value.bits % 9223372036854775808 >= 9218868437227405312 {
            perform Error.raise<JsonError>(JsonError.InvalidJson("number is not finite"));
        }
        let fixed = std.text.from_f64(value);
        var digits = "";
        var decimal: Option<usize> = None;
        var sign = "";
        for scalar in std.text.chars(fixed) {
            if scalar == '-' { sign = "-"; }
            else if scalar == '.' { decimal = Some(digits.length); }
            else { digits = digits + std.text.from_char(scalar); }
        }
        let point = match decimal { Some(at) => at, None => digits.length };
        var first: usize = 0;
        while first < digits.length && digits.utf8.get(first) == Some(48) {
            first = first + 1;
        }
        if first == digits.length { return sign + "0.0"; }
        let exponent = i32 { bits= point.bits } - i32 { bits= first.bits } - 1;
        // serde_json's pinned f64 writer uses fixed notation for [-5,15].
        // The significant digits still come from the exact source converter.
        if exponent >= -5 && exponent <= 15 {
            return if decimal.is_some() { fixed } else { fixed + ".0" };
        }
        var last = digits.length;
        while last > first + 1 && digits.utf8.get(last - 1) == Some(48) {
            last = last - 1;
        }
        var mantissa = std.text.get_byte_slice(digits, [first, first + 1)).unwrap();
        if last > first + 1 {
            mantissa = mantissa + "." + std.text.get_byte_slice(digits, [first + 1, last)).unwrap();
        }
        let suffix = if exponent >= 0 { "e+" + std.text.from_i32(exponent) } else { "e" + std.text.from_i32(exponent) };
        return sign + mantissa + suffix;
    }

    flow value(self, value: JsonValue) -> string ![Error<JsonError>] {
        return match value {
            JsonValue.Null => "null",
            JsonValue.Bool(value) => if value { "true" } else { "false" },
            JsonValue.String(value) => self.quote(value),
            JsonValue.Number(value) => self.number(value),
            JsonValue.Array(values) => {
                var text = "[";
                var separator = "";
                for value in values {
                    text = text + separator + self.value(value);
                    separator = ",";
                }
                text + "]"
            },
            JsonValue.Object(values) => {
                var ordered: std.collections.OrderedMap<string, JsonValue> = std.collections.OrderedMap.new();
                for (key, value) in values {
                    ordered = ordered.insert(key, value);
                }
                var text = "{";
                var separator = "";
                for (key, value) in ordered.iter() {
                    text = text + separator + self.quote(key) + ":" + self.value(value);
                    separator = ",";
                }
                text + "}"
            },
        };
    }
}
