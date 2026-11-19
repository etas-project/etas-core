module std.agent.session;

public spec SessionKey {
    flow stable_key(self) -> string ![];
}

impl string ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        return self;
    }
}

impl bool ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        return if self { "true" } else { "false" };
    }
}

impl i8 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_i8(self);
        return text;
    }
}

impl i16 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_i16(self);
        return text;
    }
}

impl i32 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_i32(self);
        return text;
    }
}

impl i64 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_i64(self);
        return text;
    }
}

impl i128 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_i128(self);
        return text;
    }
}

impl isize ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_isize(self);
        return text;
    }
}

impl u8 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_u8(self);
        return text;
    }
}

impl u16 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_u16(self);
        return text;
    }
}

impl u32 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_u32(self);
        return text;
    }
}

impl u64 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_u64(self);
        return text;
    }
}

impl u128 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_u128(self);
        return text;
    }
}

impl usize ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_usize(self);
        return text;
    }
}

impl f32 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_f32(self);
        return text;
    }
}

impl f64 ~ SessionKey {
    public flow stable_key(self) -> string ![] {
        let text = std.text.from_f64(self);
        return text;
    }
}
