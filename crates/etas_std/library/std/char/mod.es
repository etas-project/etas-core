module std.char;

impl char {

    public flow is_whitespace(self) -> bool ![] {
        let scalar = u32 { bits= self.bits };
        var low: usize = 0;
        var high = WHITESPACE.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let (start, end) = WHITESPACE.get(middle).unwrap();
            if scalar < start {
                high = middle;
            } else if scalar > end {
                low = middle + 1;
            } else {
                return true;
            }
        }
        return false;
    }

    public flow is_alphabetic(self) -> bool ![] {
        let scalar = u32 { bits= self.bits };
        var low: usize = 0;
        var high = ALPHABETIC.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let (start, end) = ALPHABETIC.get(middle).unwrap();
            if scalar < start {
                high = middle;
            } else if scalar > end {
                low = middle + 1;
            } else {
                return true;
            }
        }
        return false;
    }

    public flow is_alphanumeric(self) -> bool ![] {
        let scalar = u32 { bits= self.bits };
        var low: usize = 0;
        var high = ALPHANUMERIC.length;
        while low < high {
            let middle = low + (high - low) / 2;
            let (start, end) = ALPHANUMERIC.get(middle).unwrap();
            if scalar < start {
                high = middle;
            } else if scalar > end {
                low = middle + 1;
            } else {
                return true;
            }
        }
        return false;
    }

    public flow to_u32(self) -> u32 ![] {
        return u32 { bits= self.bits };
    }

    public flow from_u32(value: u32) -> Option<char> ![] {
        if value > 1114111 || (value >= 55296 && value <= 57343) {
            return None;
        }
        return Some(char { bits= value.bits });
    }

    public flow is_ascii_digit(self) -> bool ![] {
        return self.bits >= 48 && self.bits <= 57;
    }

    public flow is_control(self) -> bool ![] {
        return self.bits < 32 || (self.bits >= 127 && self.bits <= 159);
    }
}
