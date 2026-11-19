module std.text;

import std.text.TextError.{InvalidIndex, NotCharBoundary};

type BytePattern = { private pattern: bytes, private prefix: Array<usize> };

impl BytePattern {
    flow new(pattern: string) -> BytePattern ![] {
        let input = pattern.utf8;
        if input.length == 0 { return BytePattern { pattern= input, prefix= [] }; }
        var prefix: Array<usize> = [0];
        var index: usize = 1;
        var matched: usize = 0;
        while index < input.length limit Iterations(18446744073709551615) {
            let byte = input.get(index).unwrap();
            while matched > 0 && input.get(matched) != Some(byte)
                limit Iterations(18446744073709551615) {
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
        var index = start;
        var matched: usize = 0;
        while index < input.length limit Iterations(18446744073709551615) {
            let byte = input.get(index).unwrap();
            while matched > 0 && self.pattern.get(matched) != Some(byte)
                limit Iterations(18446744073709551615) {
                matched = self.prefix.get(matched - 1).unwrap();
            }
            if self.pattern.get(matched) == Some(byte) { matched = matched + 1; }
            index = index + 1;
            if matched == self.pattern.length { return Some((index - matched, index)); }
        }
        return None;
    }
}

public flow find_from<I ~ std.core.Index>(text: string, pattern: string, index: I)
    -> Option<usize> ![Error<TextError>] {
    let start = match index::std.core.Index.to_usize() {
        Some(value) => value,
        None => perform Error.raise<TextError>(InvalidIndex),
    };
    if start > text.length { perform Error.raise<TextError>(InvalidIndex); }
    if !is_char_boundary(text, start) { perform Error.raise<TextError>(NotCharBoundary); }
    let matcher = BytePattern.new(pattern);
    return match matcher.find(text.utf8, start) {
        Some((begin, _)) => Some(begin),
        None => None,
    };
}
