module std.text;

import std.text.TextError.{InvalidIndex, NotCharBoundary};

type BytePattern = { private pattern: bytes, private prefix: Array<usize> };

impl BytePattern {
    flow boundary(input: bytes, index: usize) -> bool ![] {
        if index == input.length { return true; }
        return match input.get(index) {
            Some(byte) => byte < 128 || byte >= 192,
            None => false,
        };
    }

    flow new(pattern: string) -> BytePattern ![] {
        let input = pattern.utf8;
        if input.length == 0 { return BytePattern { pattern= input, prefix= [] }; }
        var prefix: Array<usize> = [0];
        var index: usize = 1;
        var matched: usize = 0;
        while index < input.length {
            let byte = input.get(index).unwrap();
            while matched > 0 && input.get(matched) != Some(byte) {
                matched = prefix.get(matched - 1).unwrap();
            }
            if input.get(matched) == Some(byte) { matched = matched + 1; }
            prefix = prefix.push(matched);
            index = index + 1;
        }
        return BytePattern { pattern= input, prefix= prefix };
    }

    flow find(self, input: bytes, start: usize) -> Option<(usize, usize)> ![] {
        if self.pattern.length == 0 { return Some((start, start)); }
        if self.pattern.length == 1 {
            let expected = self.pattern.get(0).unwrap();
            var index = start;
            while index < input.length {
                if input.get(index) == Some(expected) { return Some((index, index + 1)); }
                index = index + 1;
            }
            return None;
        }
        var index = start;
        var matched: usize = 0;
        while index < input.length {
            let byte = input.get(index).unwrap();
            while matched > 0 && self.pattern.get(matched) != Some(byte) {
                matched = self.prefix.get(matched - 1).unwrap();
            }
            if self.pattern.get(matched) == Some(byte) { matched = matched + 1; }
            index = index + 1;
            if matched == self.pattern.length { return Some((index - matched, index)); }
        }
        return None;
    }
    flow last(self, input: bytes) -> Option<(usize, usize)> ![] {
        if self.pattern.length == 0 { return Some((input.length, input.length)); }
        var index: usize = 0;
        var matched: usize = 0;
        var found: Option<(usize, usize)> = None;
        for byte in input {
            while matched > 0 && self.pattern.get(matched) != Some(byte) {
                matched = self.prefix.get(matched - 1).unwrap();
            }
            if self.pattern.get(matched) == Some(byte) { matched = matched + 1; }
            index = index + 1;
            if matched == self.pattern.length {
                found = Some((index - matched, index));
                matched = self.prefix.get(matched - 1).unwrap();
            }
        }
        return found;
    }

}

public flow find_from<I ~ std.core.Index>(text: string, pattern: string, index: I)
    -> Option<usize> ![Error<TextError>] {
    let start = match index::std.core.Index.to_usize() {
        Some(value) => value,
        None => perform Error.raise<TextError>(InvalidIndex),
    };
    if start > text.length { perform Error.raise<TextError>(InvalidIndex); }
    let input = text.utf8;
    if !BytePattern.boundary(input, start) { perform Error.raise<TextError>(NotCharBoundary); }
    let matcher = BytePattern.new(pattern);
    return match matcher.find(input, start) {
        Some((begin, _)) => Some(begin),
        None => None,
    };
}
