module std.collections;

// Each integer range preserves its element type. The open bound is checked
// before advancing, including when the final value is the integer maximum.

impl Range<i8> ~ Iterable<i8> {
    public flow iter(self) -> Iter<i8> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<i16> ~ Iterable<i16> {
    public flow iter(self) -> Iter<i16> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<i32> ~ Iterable<i32> {
    public flow iter(self) -> Iter<i32> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<i64> ~ Iterable<i64> {
    public flow iter(self) -> Iter<i64> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<i128> ~ Iterable<i128> {
    public flow iter(self) -> Iter<i128> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<isize> ~ Iterable<isize> {
    public flow iter(self) -> Iter<isize> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<u8> ~ Iterable<u8> {
    public flow iter(self) -> Iter<u8> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<u16> ~ Iterable<u16> {
    public flow iter(self) -> Iter<u16> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<u32> ~ Iterable<u32> {
    public flow iter(self) -> Iter<u32> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<u64> ~ Iterable<u64> {
    public flow iter(self) -> Iter<u64> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<u128> ~ Iterable<u128> {
    public flow iter(self) -> Iter<u128> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}

impl Range<usize> ~ Iterable<usize> {
    public flow iter(self) -> Iter<usize> ![] {
        return Iter { next= () => {
            var first = self.start;
            if !self.include_start {
                if first >= self.end {
                    return None;
                }
                first = first + 1;
            }
            if first > self.end || (first == self.end && !self.include_end) {
                return None;
            }
            let remaining = Range {
                start= first,
                end= self.end,
                include_start= false,
                include_end= self.include_end,
            };
            return Some((first, remaining.iter()));
        } };
    }
}
