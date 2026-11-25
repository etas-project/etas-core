module std.text;

public flow from_char(value: char) -> string ![] {
    return string { scalars= [value] };
}

public flow chars(value: string) -> Iter<char> ![] {
    let input = value.utf8;
    return Iter { next= () => {
            if 0 == input.length {
                return None;
            }
            let lead = input.get(0).unwrap().bits;
            let width: usize = if lead < 128 { 1 } else if lead < 224 { 2 } else if lead < 240 { 3 } else { 4 };
            var scalar = if width == 1 { lead } else if width == 2 { lead % 32 } else if width == 3 { lead % 16 } else { lead % 8 };
            var index: usize = 1;
            while index < width {
                scalar = scalar * 64 + input.get(0 + index).unwrap().bits % 64;
                index = index + 1;
            }
            let value = char { bits= scalar };
            let remaining = ScalarCursor { input= input, offset= 0 + width };
            return Some((value, remaining.iter()));
    } };
}

type ScalarCursor = { private input: bytes, private offset: usize };

impl ScalarCursor {
    flow iter(self) -> Iter<char> ![] {
        let input = self.input;
        let offset = self.offset;
        return Iter { next= () => {
                if offset == input.length {
                    return None;
                }
                let lead = input.get(offset).unwrap().bits;
                let width: usize = if lead < 128 { 1 } else if lead < 224 { 2 } else if lead < 240 { 3 } else { 4 };
                var scalar = if width == 1 { lead } else if width == 2 { lead % 32 } else if width == 3 { lead % 16 } else { lead % 8 };
                var index: usize = 1;
                while index < width {
                    scalar = scalar * 64 + input.get(offset + index).unwrap().bits % 64;
                    index = index + 1;
                }
                let value = char { bits= scalar };
                let remaining = ScalarCursor { input= input, offset= offset + width };
                return Some((value, remaining.iter()));
        } };
    }
}
