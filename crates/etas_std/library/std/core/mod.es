module std.core;

// Terminating execution is a runtime capability; callers own their error policy.

@std.role("std.core.abort")
public flow abort(message: string) -> never ![];
public flow assert(condition: bool) -> unit ![] {
    if !condition { abort("assertion failed"); }
    return;
}

public spec Index {
    flow to_usize(self) -> Option<usize> ![];
}

impl usize {
    public flow width() -> usize ![] {
        var remaining = usize.maximum();
        var width: usize = 0;
        while remaining != 0 {
            width = width + 1;
            remaining = remaining / 2;
        }
        return width;
    }

    public flow maximum() -> usize ![] {
        return usize { bits= 340282366920938463463374607431768211455 };
    }
}

impl i8 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        if self < 0 { return None; }
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl i16 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        if self < 0 { return None; }
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl i32 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        if self < 0 { return None; }
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl i64 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        if self < 0 { return None; }
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl i128 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        if self < 0 { return None; }
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl isize ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        if self < 0 { return None; }
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl u8 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl u16 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl u32 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl u64 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl u128 ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        let index = usize { bits= self.bits };
        if index.bits != self.bits { return None; }
        return Some(index);
    }
}

impl usize ~ Index {
    public flow to_usize(self) -> Option<usize> ![] {
        return Some(self);
    }
}
