module std.compiler;

type FormatBuffer = {
    private parts: List<string>,
    private size: usize,
    private maximum: usize,
};

impl FormatBuffer {
    flow last(self) -> string ![] {
        return match self.parts {
            [] => "",
            head :: tail => head,
        };
    }

    flow append(var self, text: string) -> unit ![] {
        if text.length > self.maximum - self.size {
            std.core.abort("formatted body exceeds output byte limit");
        }
        if text != "" {
            self.parts = text :: self.parts;
            self.size = self.size + text.length;
        }
    }

    flow trim_spaces(var self) -> unit ![] {
        while self.last() == " " {
            match self.parts {
                head :: tail => {
                    self.parts = tail;
                    self.size = self.size - 1;
                },
                [] => std.core.abort("format buffer invariant"),
            };
        }
    }

    flow finish(var self) -> string ![] {
        while self.last() == " " || self.last() == "\n" {
            match self.parts {
                head :: tail => {
                    self.parts = tail;
                    self.size = self.size - 1;
                },
                [] => std.core.abort("format buffer invariant"),
            };
        }
        var output = "";
        for part in self.parts.reverse() {
            output = output + part;
        }
        return output;
    }
}

type BodyFormatter = {
    private buffer: FormatBuffer,
    private indent: usize,
    private previous: Option<SyntaxToken>,
};

impl BodyFormatter {
    flow write(var self, text: string) -> unit ![] {
        var buffer = self.buffer;
        if buffer.last() == "\n" {
            var level: usize = 0;
            while level < self.indent {
                buffer.append("    ");
                level = level + 1;
            }
        }
        buffer.append(text);
        self.buffer = buffer;
    }

    flow space(var self) -> unit ![] {
        let last = self.buffer.last();
        if last != "" && last != " " && last != "\n" {
            self.write(" ");
        }
    }

    flow newline(var self) -> unit ![] {
        var buffer = self.buffer;
        buffer.trim_spaces();
        let last = buffer.last();
        if last != "" && last != "\n" {
            buffer.append("\n");
        }
        self.buffer = buffer;
    }

    flow token(var self, token: SyntaxToken, next: Option<SyntaxToken>) -> unit ![] {
        let kind = token.kind;
        let text = token.text;
        if kind == SyntaxTokenKind.Punctuation && text == "{" {
            if self.previous.is_some() && !self.previous.is_punctuation("#") {
                self.space();
            }
            self.write(text);
            self.indent = self.indent + 1;
            if !next.is_punctuation("}") { self.newline(); }
        } else if kind == SyntaxTokenKind.Punctuation && text == "}" {
            if self.indent > 0 { self.indent = self.indent - 1; }
            if !self.previous.is_punctuation("{") { self.newline(); }
            self.write(text);
        } else if kind == SyntaxTokenKind.Punctuation && text == ";" {
            var buffer = self.buffer;
            buffer.trim_spaces();
            self.buffer = buffer;
            self.write(text);
            if !next.is_eof() { self.newline(); }
        } else if kind == SyntaxTokenKind.Punctuation && (text == "," || text == ":") {
            var buffer = self.buffer;
            buffer.trim_spaces();
            self.buffer = buffer;
            self.write(text);
            self.space();
        } else if kind == SyntaxTokenKind.Punctuation &&
            (text == "." || text == "::" || text == "?" || text == ")" || text == "]") {
            var buffer = self.buffer;
            buffer.trim_spaces();
            self.buffer = buffer;
            self.write(text);
        } else if kind == SyntaxTokenKind.Punctuation && (text == "(" || text == "[") {
            if self.previous.is_keyword("return") || self.previous.is_keyword("if") || self.previous.is_keyword("match") {
                self.space();
            }
            self.write(text);
        } else {
            if self.previous.is_punctuation("}") &&
                !(kind == SyntaxTokenKind.Keyword && (text == "else" || text == "with")) &&
                (kind == SyntaxTokenKind.Identifier || kind == SyntaxTokenKind.Keyword) {
                self.newline();
            }
            if self.previous.needs_space() { self.space(); }
            self.write(text);
        }
        self.previous = Some(token);
    }
}

impl Option<SyntaxToken> {
    flow is_punctuation(self, text: string) -> bool ![] {
        return match self {
            None => false,
            Some(token) => token.kind == SyntaxTokenKind.Punctuation && token.text == text,
        };
    }

    flow is_keyword(self, text: string) -> bool ![] {
        return match self {
            None => false,
            Some(token) => token.kind == SyntaxTokenKind.Keyword && token.text == text,
        };
    }

    flow is_eof(self) -> bool ![] {
        return match self {
            None => false,
            Some(token) => token.kind == SyntaxTokenKind.Eof,
        };
    }

    flow needs_space(self) -> bool ![] {
        return match self {
            None => false,
            Some(token) => {
                if token.kind != SyntaxTokenKind.Punctuation { return true; }
                let text = token.text;
                return text != "(" && text != "[" && text != "." && text != "::" &&
                    text != "^" && text != "!" && text != "#";
            },
        };
    }
}
