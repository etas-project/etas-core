module std.collections;

public spec Eq;
public spec Hash ~ Eq {
    flow hash(self, state: Hasher) -> Hasher ![];
}

// FNV-1a/64 profile. Seed mixing and byte framing are source algorithms.
public type Hasher = { private state: u64 };
impl Hasher {
    public flow new(seed: u64 = 0) -> Hasher ![] {
        var left: u128 = 14695981039346656037;
        var right = seed.bits;
        var mixed: u128 = 0;
        var place: u128 = 1;
        var bit: usize = 0;
        while bit < 64 {
            mixed = mixed + ((left % 2 + right % 2) % 2) * place;
            left = left / 2; right = right / 2; place = place * 2; bit = bit + 1;
        }
        return Hasher { state= u64 { bits= mixed } };
    }
    public flow write(self, data: bytes) -> Hasher ![] {
        var state = self.state.bits;
        for byte in data {
            var a = state % 256; var b = byte.bits;
            var low: u128 = 0; var place: u128 = 1; var bit: usize = 0;
            while bit < 8 {
                low = low + ((a % 2 + b % 2) % 2) * place;
                a = a / 2; b = b / 2; place = place * 2; bit = bit + 1;
            }
            state = ((state - state % 256 + low) * 1099511628211) % 18446744073709551616;
        }
        return Hasher { state= u64 { bits= state } };
    }
    public flow finish(self) -> u64 ![] { return self.state; }
}

impl i8 ~ Eq {}
impl i8 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 1 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl i16 ~ Eq {}
impl i16 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 2 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl i32 ~ Eq {}
impl i32 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 4 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl i64 ~ Eq {}
impl i64 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 8 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl i128 ~ Eq {}
impl i128 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 16 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl isize ~ Eq {}
impl isize ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var remaining_width = usize.maximum().bits;
        var offset: usize = 0;
        while remaining_width != 0 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; remaining_width = remaining_width / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl u8 ~ Eq {}
impl u8 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 1 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl u16 ~ Eq {}
impl u16 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 2 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl u32 ~ Eq {}
impl u32 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 4 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl u64 ~ Eq {}
impl u64 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 8 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl u128 ~ Eq {}
impl u128 ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 16 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl usize ~ Eq {}
impl usize ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var remaining_width = usize.maximum().bits;
        var offset: usize = 0;
        while remaining_width != 0 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; remaining_width = remaining_width / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl char ~ Eq {}
impl char ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        var data: Array<u8> = [];
        var bits = self.bits;
        var offset: usize = 0;
        while offset < 4 {
            data = data.push(u8 { bits= bits % 256 });
            bits = bits / 256; offset = offset + 1;
        }
        return state.write(bytes { data= data });
    }
}

impl bool ~ Eq {}
impl bool ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        return state.write(bytes { data= [if self { 1 } else { 0 }] });
    }
}
impl string ~ Eq {}
impl string ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        let framed = self.length.hash(state);
        return framed.write(self.utf8);
    }
}
impl bytes ~ Eq {}
impl bytes ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        let framed = self.length.hash(state);
        return framed.write(self);
    }
}
impl unit ~ Eq {}
impl unit ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        return state.write(bytes { data= [0] });
    }
}
