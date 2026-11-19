module std.agent.session;

import std.memory.StorageError;
import std.json.JsonValue;


// Fence integer tokens stay lossless. Going through JsonValue.Number(f64)
// would round i64 history ordinals and the optional i128 retention cutoff.
enum FenceValue {
    Null,
    Bool(bool),
    Number(string),
    String(string),
    Array(Array<FenceValue>),
    Object(Map<string, FenceValue>),
}

type FenceReader = { private text: string, private position: usize, private max_depth: usize };

impl FenceReader {
    flow fail(self) -> never ![Error<StorageError>] {
        perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid session history fence" });
    }

    flow whitespace(var self) -> unit ![] {
        while self.position < self.text.length limit Iterations(18446744073709551615) {
            let byte = self.text.utf8.get(self.position).unwrap();
            if byte != 32 && byte != 9 && byte != 10 && byte != 13 { return; }
            self.position = self.position + 1;
        }
    }

    flow peek(self) -> Option<u8> ![] {
        return self.text.utf8.get(self.position);
    }

    flow expect(var self, byte: u8) -> unit ![Error<StorageError>] {
        if self.peek() != Some(byte) { self.fail(); }
        self.position = self.position + 1;
    }

    flow quoted(var self) -> string ![Error<StorageError>] {
        let start = self.position;
        self.expect(34);
        while self.position < self.text.length limit Iterations(18446744073709551615) {
            let byte = self.text.utf8.get(self.position).unwrap();
            self.position = self.position + 1;
            if byte == 92 {
                if self.position == self.text.length { self.fail(); }
                self.position = self.position + 1;
            } else if byte == 34 {
                let token = std.text.get_byte_slice(self.text, [start, self.position)).unwrap();
                return match std.json.parse(token) {
                    Ok(JsonValue.String(value)) => value,
                    _ => self.fail(),
                };
            }
        }
        self.fail();
    }

    flow literal(var self, expected: string) -> unit ![Error<StorageError>] {
        let end = self.position + expected.length;
        if std.text.get_byte_slice(self.text, [self.position, end)) != Some(expected) { self.fail(); }
        self.position = end;
    }

    flow value(var self, depth: usize) -> FenceValue ![Error<StorageError>] {
        if depth > self.max_depth {
            perform Error.raise<StorageError>(StorageError { code= "budget_exceeded", message= "session history fence exceeds depth limit" });
        }
        self.whitespace();
        return match self.peek() {
            Some(34) => FenceValue.String(self.quoted()),
            Some(110) => { self.literal("null"); FenceValue.Null },
            Some(116) => { self.literal("true"); FenceValue.Bool(true) },
            Some(102) => { self.literal("false"); FenceValue.Bool(false) },
            Some(91) => {
                self.position = self.position + 1;
                self.whitespace();
                var items: Array<FenceValue> = [];
                if self.peek() == Some(93) { self.position = self.position + 1; return FenceValue.Array(items); }
                while true limit Iterations(18446744073709551615) {
                    items = items.push(self.value(depth + 1));
                    self.whitespace();
                    if self.peek() == Some(93) { self.position = self.position + 1; return FenceValue.Array(items); }
                    self.expect(44); self.whitespace();
                }
                self.fail()
            },
            Some(123) => {
                self.position = self.position + 1;
                self.whitespace();
                var fields: Map<string, FenceValue> = {};
                if self.peek() == Some(125) { self.position = self.position + 1; return FenceValue.Object(fields); }
                while true limit Iterations(18446744073709551615) {
                    let name = self.quoted();
                    if fields.contains_key(name) { self.fail(); }
                    self.whitespace(); self.expect(58);
                    fields = fields.insert(name, self.value(depth + 1));
                    self.whitespace();
                    if self.peek() == Some(125) { self.position = self.position + 1; return FenceValue.Object(fields); }
                    self.expect(44); self.whitespace();
                }
                self.fail()
            },
            Some(_) => {
                let start = self.position;
                while self.position < self.text.length limit Iterations(18446744073709551615) {
                    let byte = self.text.utf8.get(self.position).unwrap();
                    if (byte >= 48 && byte <= 57) || byte == 45 || byte == 43 || byte == 46 || byte == 101 || byte == 69 {
                        self.position = self.position + 1;
                    } else { break; }
                }
                let token = std.text.get_byte_slice(self.text, [start, self.position)).unwrap();
                // Integer grammar is checked without a floating conversion so
                // a valid i128 cutoff does not acquire an f64 rounding step.
                let integral = token != "" && !std.text.contains(token, ".") && !std.text.contains(token, "e") && !std.text.contains(token, "E");
                if integral {
                    let digits = if std.text.starts_with(token, "-") { std.text.get_byte_slice(token, [1, token.length)).unwrap() } else { token };
                    if digits == "" || (digits.length > 1 && std.text.starts_with(digits, "0")) { self.fail(); }
                    for byte in digits.utf8 limit Iterations(18446744073709551615) {
                        if byte < 48 || byte > 57 { self.fail(); }
                    }
                } else {
                    match std.json.parse(token) { Ok(JsonValue.Number(_)) => {}, _ => self.fail(), };
                }
                FenceValue.Number(token)
            },
            None => self.fail(),
        };
    }
}

impl FenceValue {
    flow parse(text: string, max_depth: usize) -> FenceValue ![Error<StorageError>] {
        var reader = FenceReader { text= text, position= 0, max_depth= max_depth };
        let value = reader.value(0);
        reader.whitespace();
        if reader.position != text.length { reader.fail(); }
        return value;
    }

    flow invalid(self) -> never ![Error<StorageError>] {
        perform Error.raise<StorageError>(StorageError { code= "invalid_request", message= "invalid session history fence" });
    }

    flow object(self, required: Array<string>, optional: Array<string>) -> Map<string, FenceValue> ![Error<StorageError>] {
        let fields = match self { FenceValue.Object(fields) => fields, _ => self.invalid(), };
        var allowed: std.collections.OrderedSet<string> = std.collections.OrderedSet.new();
        for name in required limit Iterations(32) {
            if !fields.contains_key(name) { self.invalid(); }
            allowed = allowed.insert(name);
        }
        for name in optional limit Iterations(32) { allowed = allowed.insert(name); }
        for (name, value) in fields limit Iterations(32) { if !allowed.contains(name) { self.invalid(); } }
        return fields;
    }

    flow text(self) -> string ![Error<StorageError>] {
        return match self { FenceValue.String(value) => value, _ => self.invalid(), };
    }

    flow integer(self) -> i64 ![Error<StorageError>] {
        let token = match self { FenceValue.Number(value) => value, _ => self.invalid(), };
        if token == "-0" { self.invalid(); }
        return match std.text.parse_i64(token) { Ok(value) => value, Err(_) => self.invalid(), };
    }

    flow unsigned(self) -> u64 ![Error<StorageError>] {
        let token = match self { FenceValue.Number(value) => value, _ => self.invalid(), };
        return match std.text.parse_u64(token) { Ok(value) => value, Err(_) => self.invalid(), };
    }

    flow integer128(self) -> i128 ![Error<StorageError>] {
        let token = match self { FenceValue.Number(value) => value, _ => self.invalid(), };
        return match std.text.parse_i128(token) { Ok(value) => value, Err(_) => self.invalid(), };
    }

    flow hex(self, width: usize) -> unit ![Error<StorageError>] {
        let text = self.text();
        if text.length != width { self.invalid(); }
        for byte in text.utf8 limit Iterations(64) {
            if !((byte >= 48 && byte <= 57) || (byte >= 97 && byte <= 102)) { self.invalid(); }
        }
    }
}
