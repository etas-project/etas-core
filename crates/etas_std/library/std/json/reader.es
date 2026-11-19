module std.json;

type JsonReader = { private input: Array<char>, private position: usize };

impl JsonReader {
    flow fail(self, message: string) -> never ![Error<JsonError>] {
        perform Error.raise<JsonError>(JsonError.InvalidJson(
            message + " at character " + std.text.from_usize(self.position)
        ));
    }

    flow peek(self) -> Option<char> ![] {
        return self.input.get(self.position);
    }

    flow take(var self) -> char ![Error<JsonError>] {
        let scalar = match self.input.get(self.position) {
            Some(value) => value,
            None => self.fail("unexpected end of JSON"),
        };
        self.position = self.position + 1;
        return scalar;
    }

    flow whitespace(var self) -> unit ![] {
        while true limit Iterations(18446744073709551615) {
            match self.peek() {
                Some(' ') => {},
                Some('\t') => {},
                Some('\r') => {},
                Some('\n') => {},
                _ => { return; },
            };
            self.position = self.position + 1;
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
        if self.position != self.input.length {
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
        for scalar in std.text.chars(spelling) limit Iterations(18446744073709551615) {
            self.expect(scalar);
        }
        return value;
    }

    flow string(var self) -> string ![Error<JsonError>] {
        self.expect('"');
        var output = "";
        while true limit Iterations(18446744073709551615) {
            let scalar = self.take();
            if scalar == '"' { return output; }
            if scalar == '\\' {
                let escaped = self.take();
                output = output + match escaped {
                    '"' => "\"",
                    '\\' => "\\",
                    '/' => "/",
                    'b' => "\u{0008}",
                    'f' => "\u{000c}",
                    'n' => "\n",
                    'r' => "\r",
                    't' => "\t",
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
                        std.text.from_char(decoded)
                    },
                    _ => self.fail("invalid JSON string escape"),
                };
            } else {
                if scalar.to_u32() < 32 {
                    self.fail("unescaped control character in JSON string");
                }
                output = output + std.text.from_char(scalar);
            }
        }
    }

    flow hex_quad(var self) -> u32 ![Error<JsonError>] {
        var code: u32 = 0;
        var count: usize = 0;
        while count < 4 limit Iterations(4) {
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
            self.position = self.position + 1;
            return JsonValue.Array(output);
        }
        while true limit Iterations(18446744073709551615) {
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
            self.position = self.position + 1;
            return JsonValue.Object(output);
        }
        while true limit Iterations(18446744073709551615) {
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

    flow digits(var self) -> string ![] {
        var text = "";
        while true limit Iterations(18446744073709551615) {
            let scalar = match self.peek() {
                Some(value) => value,
                None => { return text; },
            };
            if scalar < '0' || scalar > '9' { return text; }
            text = text + std.text.from_char(scalar);
            self.position = self.position + 1;
        }
    }

    flow number(var self) -> JsonValue ![Error<JsonError>] {
        var text = "";
        if self.peek() == Some('-') {
            text = "-";
            self.position = self.position + 1;
        }
        let initial = self.take();
        if initial == '0' {
            text = text + "0";
        } else if initial >= '1' && initial <= '9' {
            text = text + std.text.from_char(initial) + self.digits();
        } else {
            self.fail("expected integer part of JSON number");
        }
        if self.peek() == Some('.') {
            self.position = self.position + 1;
            let fraction = self.digits();
            if fraction == "" { self.fail("expected fraction digits"); }
            text = text + "." + fraction;
        }
        let exponent = self.peek();
        if exponent == Some('e') || exponent == Some('E') {
            self.position = self.position + 1;
            text = text + "e";
            let sign = self.peek();
            if sign == Some('+') || sign == Some('-') {
                text = text + std.text.from_char(self.take());
            }
            let digits = self.digits();
            if digits == "" { self.fail("expected exponent digits"); }
            text = text + digits;
        }
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
