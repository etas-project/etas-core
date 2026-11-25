module std.bytes;

public type BytesBuilder = {
    private chunks: List<bytes>,
    private length: usize,
};

impl BytesBuilder {
    public flow new() -> BytesBuilder ![] {
        return BytesBuilder { chunks= [], length= 0 };
    }
    public flow append(var self, part: bytes) -> unit ![] {
        self = BytesBuilder { chunks= part :: self.chunks, length= self.length + part.length };
    }
    public flow push(var self, value: u8) -> unit ![] {
        self = BytesBuilder { chunks= bytes { data= [value] } :: self.chunks, length= self.length + 1 };
    }
    public flow len_bytes(self) -> usize ![] {
        return self.length;
    }
    public flow is_empty(self) -> bool ![] {
        return self.length == 0;
    }
    public flow clear(var self) -> unit ![] {
        self = BytesBuilder { chunks= [], length= 0 };
    }
    public flow build(self) -> bytes ![] {
        var output: Array<u8> = [];
        for chunk in self.chunks.reverse() {
            for value in chunk { output = output.push(value); }
        }
        return bytes { data= output };
    }
}
