module std.collections;

public spec Order { flow compare(self, other: Self) -> i32 ![]; }

impl i8 ~ Order {
    public flow compare(self, other: i8) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl i16 ~ Order {
    public flow compare(self, other: i16) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl i32 ~ Order {
    public flow compare(self, other: i32) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl i64 ~ Order {
    public flow compare(self, other: i64) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl i128 ~ Order {
    public flow compare(self, other: i128) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl isize ~ Order {
    public flow compare(self, other: isize) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl u8 ~ Order {
    public flow compare(self, other: u8) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl u16 ~ Order {
    public flow compare(self, other: u16) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl u32 ~ Order {
    public flow compare(self, other: u32) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl u64 ~ Order {
    public flow compare(self, other: u64) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl u128 ~ Order {
    public flow compare(self, other: u128) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl usize ~ Order {
    public flow compare(self, other: usize) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl char ~ Order {
    public flow compare(self, other: char) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl string ~ Order {
    public flow compare(self, other: string) -> i32 ![] {
        if self < other { return -1; }
        if self > other { return 1; }
        return 0;
    }
}

impl bool ~ Order {
    public flow compare(self, other: bool) -> i32 ![] {
        if self == other { return 0; }
        return if self { 1 } else { -1 };
    }
}

impl f32 ~ Order {
    public flow compare(self, other: f32) -> i32 ![] {
        let left = self.bits;
        let right = other.bits;
        if left == right { return 0; }
        let negative_left = left >= 2147483648;
        let negative_right = right >= 2147483648;
        if negative_left != negative_right { return if negative_left { -1 } else { 1 }; }
        if negative_left { return if left > right { -1 } else { 1 }; }
        return if left < right { -1 } else { 1 };
    }
}

impl f64 ~ Order {
    public flow compare(self, other: f64) -> i32 ![] {
        let left = self.bits;
        let right = other.bits;
        if left == right { return 0; }
        let negative_left = left >= 9223372036854775808;
        let negative_right = right >= 9223372036854775808;
        if negative_left != negative_right { return if negative_left { -1 } else { 1 }; }
        if negative_left { return if left > right { -1 } else { 1 }; }
        return if left < right { -1 } else { 1 };
    }
}

