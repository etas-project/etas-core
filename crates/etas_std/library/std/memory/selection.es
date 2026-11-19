module std.memory;

import std.runtime.value.Data;
import std.agent.prompt.{PromptEncode, PromptPart};
import std.json.JsonValue;


// This V bound is a conditional witness requirement. The compiler must retain
// it (and the captured Store place), not publish an unconstrained blanket impl.
impl<V ~ PromptEncode> MemorySelection<V> ~ PromptEncode {
    public flow encode(self) -> PromptPart ![] {
        let entries = (self.evaluate)(self.maximum);
        var output: Array<JsonValue> = [];
        for entry in entries limit Iterations(18446744073709551615) {
            let row = Data.Record([
                ("key", entry.key), ("value", entry.value),
                ("version", Data.String(entry.version.opaque)),
            ]);
            match row.to_json() {
                Ok(value) => { output = output.push(value); },
                Err(error) => std.core.abort(error.message),
            };
        }
        let text = match std.json.stringify(JsonValue.Array(output)) {
            Ok(text) => text,
            Err(_) => std.core.abort("memory selection contains a non-JSON value"),
        };
        return PromptPart { text= text };
    }
}

impl Data {
    flow embedding(self) -> Array<f32> ![] {
        let values = match self {
            Data.List(values) => values,
            _ => std.core.abort("related_to requires a nonempty numeric embedding"),
        };
        if values.length == 0 { std.core.abort("related_to requires a nonempty numeric embedding"); }
        var output: Array<f32> = [];
        for value in values limit Iterations(18446744073709551615) {
            let number = match value {
                Data.Float(value) => {
                    if value.bits % 9223372036854775808 >= 9218868437227405312 {
                        std.core.abort("related_to requires a finite numeric embedding");
                    }
                    EmbeddingNumber.from_f64(value).as_f32()
                },
                Data.Int(value) => match std.text.parse_f32(std.text.from_i128(value)) {
                    Ok(value) => value,
                    Err(_) => f32 { bits= if value < 0 { 4286578688 } else { 2139095040 } },
                },
                Data.UInt(value) => match std.text.parse_f32(std.text.from_u128(value)) {
                    Ok(value) => value,
                    Err(_) => f32 { bits= 2139095040 },
                },
                _ => std.core.abort("related_to requires a numeric embedding"),
            };
            output = output.push(number);
        }
        return output;
    }
}

// Explicit IEEE f64 -> f32 conversion in source, including ordinary underflow
// to signed zero. Decimal parse's nonzero-underflow error is not a cast rule.
type EmbeddingNumber = { private coefficient: u128, private power: i32, private negative: bool };

impl EmbeddingNumber {
    flow from_f64(value: f64) -> EmbeddingNumber ![] {
        let magnitude = value.bits % 9223372036854775808;
        let exponent = magnitude / 4503599627370496;
        let fraction = magnitude % 4503599627370496;
        return EmbeddingNumber {
            coefficient= fraction + if exponent == 0 { 0 } else { 4503599627370496 },
            power= if exponent == 0 { -1074 } else { i32 { bits= exponent } - 1075 },
            negative= value.bits >= 9223372036854775808,
        };
    }

    flow rounded(self, shift: i32) -> u128 ![] {
        if shift > 54 { return 0; }
        var result = self.coefficient;
        if shift <= 0 {
            var remaining = 0 - shift;
            while remaining > 0 limit Iterations(54) { result = result * 2; remaining = remaining - 1; }
            return result;
        }
        var divisor: u128 = 1;
        var count: i32 = 0;
        while count < shift limit Iterations(54) { divisor = divisor * 2; count = count + 1; }
        let remainder = result % divisor;
        result = result / divisor;
        if remainder * 2 > divisor || (remainder * 2 == divisor && result % 2 == 1) { result = result + 1; }
        return result;
    }

    flow as_f32(self) -> f32 ![] {
        let sign: u128 = if self.negative { 2147483648 } else { 0 };
        if self.coefficient == 0 { return f32 { bits= sign }; }
        var length: i32 = 0;
        var value = self.coefficient;
        while value > 0 limit Iterations(53) { value = value / 2; length = length + 1; }
        var exponent = self.power + length - 1;
        if exponent < -126 {
            return f32 { bits= sign + self.rounded(0 - self.power - 149) };
        }
        if exponent > 127 { return f32 { bits= sign + 2139095040 }; }
        var coefficient = self.rounded(length - 24);
        if coefficient == 16777216 { coefficient = coefficient / 2; exponent = exponent + 1; }
        if exponent > 127 { return f32 { bits= sign + 2139095040 }; }
        return f32 { bits= sign + (exponent + 127).bits * 8388608 + coefficient - 8388608 };
    }
}
