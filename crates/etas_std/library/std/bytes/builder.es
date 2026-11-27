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
        // Fold pairs of adjacent chunks so each byte is copied at most once per level.
        var chunks = self.chunks;
        while true {
            var pairs: List<bytes> = [];
            match chunks {
                [] => { return bytes { data= [] }; },
                newest :: [] => { return newest; },
                _ => {},
            };
            while true {
                match chunks {
                    newest :: older :: tail => {
                        pairs = (older + newest) :: pairs;
                        chunks = tail;
                    },
                    last :: [] => { pairs = last :: pairs; break; },
                    [] => { break; },
                };
            }
            chunks = pairs.reverse();
        }
        return bytes { data= [] };
    }
}
