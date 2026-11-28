module std.json;

type JsonReader = { private input: bytes, private offset: usize, private position: usize };

impl JsonReader {
    flow fail(self, message: string) -> never ![Error<JsonError>] {
        perform Error.raise<JsonError>(JsonError.InvalidJson(
            message + " at character " + std.text.from_usize(self.position)
        ));
    }

    flow peek(self) -> Option<char> ![] {
        let lead = match self.input.get(self.offset) { None => { return None; }, Some(byte) => byte.bits };
        if lead < 128 { return Some(char { bits= lead }); }
        let width: usize = if lead < 224 { 2 } else if lead < 240 { 3 } else { 4 };
        var scalar = lead % (if width == 2 { 32 } else if width == 3 { 16 } else { 8 });
        var index: usize = 1;
        while index < width {
            scalar = scalar * 64 + self.input.get(self.offset + index).unwrap().bits % 64;
            index = index + 1;
        }
        return Some(char { bits= scalar });
    }

    flow advance_ascii(var self) -> unit ![] {
        self.offset = self.offset + 1;
        self.position = self.position + 1;
    }

    flow take(var self) -> char ![Error<JsonError>] {
        let scalar = match self.peek() {
            Some(value) => value,
            None => self.fail("unexpected end of JSON"),
        };
        let width: usize = if scalar.bits < 128 { 1 } else if scalar.bits < 2048 { 2 } else if scalar.bits < 65536 { 3 } else { 4 };
        self.offset = self.offset + width;
        self.position = self.position + 1;
        return scalar;
    }

    flow whitespace(var self) -> unit ![] {
        while true {
            match self.peek() {
                Some(' ') => {},
                Some('\t') => {},
                Some('\r') => {},
                Some('\n') => {},
                _ => { return; },
            };
            self.advance_ascii();
        }
    }

    flow expect(var self, expected: char) -> unit ![Error<JsonError>] {
        if self.take() != expected {
            self.fail("unexpected JSON character");
        }
    }

    flow document(var self) -> JsonValue ![Error<JsonError>] {
        let value = self.value();
        self.whitespace();
        if self.offset != self.input.length {
            self.fail("trailing characters after JSON value");
        }
        return value;
    }

    flow value(var self) -> JsonValue ![Error<JsonError>] {
        self.whitespace();
        return match self.peek() {
            Some('n') => self.literal("null", JsonValue.Null),
            Some('t') => self.literal("true", JsonValue.Bool(true)),
            Some('f') => self.literal("false", JsonValue.Bool(false)),
            Some('"') => JsonValue.String(self.string()),
            Some('[') => self.array(),
            Some('{') => self.object(),
            Some('-') => self.number(),
            Some(digit) => {
                if digit >= '0' && digit <= '9' {
                    self.number()
                } else {
                    self.fail("expected a JSON value")
                }
            },
            None => self.fail("expected a JSON value"),
        };
    }

    flow literal(var self, spelling: string, value: JsonValue) -> JsonValue ![Error<JsonError>] {
        for scalar in std.text.chars(spelling) {
            self.expect(scalar);
        }
        return value;
    }

    flow string(var self) -> string ![Error<JsonError>] {
        self.expect('"');
        var scalars: Array<char> = [];
        var text = "";
        while true {
            if scalars.length == 64 {
                text = text + string { scalars= scalars };
                scalars = [];
            }
            let scalar = self.take();
            if scalar == '"' { return text + string { scalars= scalars }; }
            if scalar == '\\' {
                let escaped = self.take();
                let decoded = match escaped {
                    '"' => char { bits= 34 },
                    '\\' => char { bits= 92 },
                    '/' => char { bits= 47 },
                    'b' => char { bits= 8 },
                    'f' => char { bits= 12 },
                    'n' => char { bits= 10 },
                    'r' => char { bits= 13 },
                    't' => char { bits= 9 },
                    'u' => {
                        var code = self.hex_quad();
                        if code >= 55296 && code <= 56319 {
                            self.expect('\\');
                            self.expect('u');
                            let low = self.hex_quad();
                            if low < 56320 || low > 57343 {
                                self.fail("expected a low Unicode surrogate");
                            }
                            code = 65536 + (code - 55296) * 1024 + low - 56320;
                        }
                        let decoded = match char.from_u32(code) {
                            Some(value) => value,
                            None => self.fail("invalid Unicode scalar in JSON escape"),
                        };
                        decoded
                    },
                    _ => self.fail("invalid JSON string escape"),
                };
                scalars = scalars + [decoded];
            } else {
                if scalar.to_u32() < 32 {
                    self.fail("unescaped control character in JSON string");
                }
                scalars = scalars + [scalar];
            }
        }
    }

    flow hex_quad(var self) -> u32 ![Error<JsonError>] {
        var code: u32 = 0;
        var count: usize = 0;
        while count < 4 {
            let scalar = self.take();
            let digit = scalar.to_u32();
            code = code * 16 + if scalar >= '0' && scalar <= '9' {
                digit - 48
            } else if scalar >= 'a' && scalar <= 'f' {
                digit - 87
            } else if scalar >= 'A' && scalar <= 'F' {
                digit - 55
            } else {
                self.fail("invalid hexadecimal JSON escape")
            };
            count = count + 1;
        }
        return code;
    }

    flow array(var self) -> JsonValue ![Error<JsonError>] {
        self.expect('[');
        self.whitespace();
        var output: Array<JsonValue> = [];
        if self.peek() == Some(']') {
            self.advance_ascii();
            return JsonValue.Array(output);
        }
        while true {
            output = output.push(self.value());
            self.whitespace();
            match self.take() {
                ']' => { return JsonValue.Array(output); },
                ',' => {},
                _ => self.fail("expected comma or closing array bracket"),
            };
        }
    }

    flow object(var self) -> JsonValue ![Error<JsonError>] {
        self.expect('{');
        self.whitespace();
        var output: Map<string, JsonValue> = {};
        if self.peek() == Some('}') {
            self.advance_ascii();
            return JsonValue.Object(output);
        }
        while true {
            self.whitespace();
            let key = self.string();
            self.whitespace();
            self.expect(':');
            let value = self.value();
            output = output.insert(key, value);
            self.whitespace();
            match self.take() {
                '}' => { return JsonValue.Object(output); },
                ',' => {},
                _ => self.fail("expected comma or closing object brace"),
            };
        }
    }

    flow digits(var self) -> usize ![] {
        let start = self.offset;
        while true {
            match self.input.get(self.offset) {
                Some(byte) => { if byte < 48 || byte > 57 { break; } },
                None => { break; },
            };
            self.advance_ascii();
        }
        return self.offset - start;
    }

    flow number(var self) -> JsonValue ![Error<JsonError>] {
        let start = self.offset;
        if self.peek() == Some('-') { self.advance_ascii(); }
        let initial = self.take();
        if initial >= '1' && initial <= '9' { self.digits(); }
        else if initial != '0' { self.fail("expected integer part of JSON number"); }
        if self.peek() == Some('.') {
            self.advance_ascii();
            if self.digits() == 0 { self.fail("expected fraction digits"); }
        }
        let exponent = self.peek();
        if exponent == Some('e') || exponent == Some('E') {
            self.advance_ascii();
            let sign = self.peek();
            if sign == Some('+') || sign == Some('-') { self.advance_ascii(); }
            if self.digits() == 0 { self.fail("expected exponent digits"); }
        }
        let token = self.input[start, self.offset)?.unwrap();
        let text = std.codec.text.utf8_decode(token, std.codec.text.Strict).unwrap();
        let value = match std.text.parse_f64(text) {
            Ok(value) => value,
            Err(_) => self.fail("JSON number is outside f64 range"),
        };
        if value.bits % 9223372036854775808 >= 9218868437227405312 {
            self.fail("JSON number is not finite");
        }
        return JsonValue.Number(value);
    }
}
