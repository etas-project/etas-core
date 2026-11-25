module std.text;

public type TextBuilder = {
    private chunks: List<string>,
    private length: usize,
};

impl TextBuilder {
    public flow new() -> TextBuilder ![] {
        return TextBuilder { chunks= [], length= 0 };
    }
    public flow append(var self, part: string) -> unit ![] {
        self = TextBuilder { chunks= part :: self.chunks, length= self.length + part.length };
    }
    public flow append_char(var self, value: char) -> unit ![] {
        let part = from_char(value);
        self = TextBuilder { chunks= part :: self.chunks, length= self.length + part.length };
    }
    public flow len_bytes(self) -> usize ![] {
        return self.length;
    }
    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }
    public flow clear(var self) -> unit ![] {
        self = TextBuilder { chunks= [], length= 0 };
    }
    public flow build(self) -> string ![] {
        var output: Array<char> = [];
        for chunk in self.chunks.reverse() {
            for value in chars(chunk) {
                output = output.push(value);
            }
        }
        return string { scalars= output };
    }
}
