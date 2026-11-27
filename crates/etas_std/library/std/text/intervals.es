module std.text;

import std.text.TextError.{InvalidRange, LengthOverflow, NotCharBoundary};

import std.core.Index;


impl Range<I> {
    flow text_bounds(self) -> Result<(usize, usize), TextError> ![] {
        var start = match self.start::Index.to_usize() {
            Some(value) => value,
            None => { return Err(InvalidRange); },
        };
        var end = match self.end::Index.to_usize() {
            Some(value) => value,
            None => { return Err(InvalidRange); },
        };
        let maximum = usize.maximum();
        if !self.include_start {
            if start == maximum { return Err(InvalidRange); }
            start = start + 1;
        }
        if self.include_end {
            if end == maximum { return Err(InvalidRange); }
            end = end + 1;
        }
        if start > end { return Err(InvalidRange); }
        return Ok((start, end));
    }
}

type TextSpan = { private input: bytes, private start: usize, private end: usize };

impl TextSpan {
    flow materialize(self) -> string ![] {
        let view = self.input[self.start, self.end)?.unwrap();
        return match std.codec.text.utf8_decode(view, std.codec.text.Strict) {
            Ok(text) => text,
            Err(_) => std.core.abort("text span UTF-8 invariant"),
        };
    }
}

public flow byte_slice<I ~ Index>(text: string, range: Range<I>) -> string ![Error<TextError>] {
    let (start, end) = match range.text_bounds() {
        Ok(value) => value,
        Err(error) => perform Error.raise<TextError>(error),
    };
    if end > text.length { perform Error.raise<TextError>(InvalidRange); }
    let input = text.utf8;
    if !BytePattern.boundary(input, start) || !BytePattern.boundary(input, end) {
        perform Error.raise<TextError>(NotCharBoundary);
    }
    return TextSpan { input= input, start= start, end= end }.materialize();
}

public flow char_slice<I ~ Index>(text: string, range: Range<I>) -> string ![Error<TextError>] {
    let (start, end) = match range.text_bounds() {
        Ok(value) => value,
        Err(error) => perform Error.raise<TextError>(error),
    };
    if end == 0 { return ""; }
    var ordinal: usize = 0;
    let input = text.utf8;
    var begin: usize = 0;
    var offset: usize = 0;
    for scalar in (ScalarCursor { input= input, offset= 0 }.iter()) {
        if ordinal == start { begin = offset; }
        offset = offset + scalar_width(scalar);
        ordinal = ordinal + 1;
        if ordinal == end { return TextSpan { input= input, start= if start == end { offset } else { begin }, end= offset }.materialize(); }
    }
    perform Error.raise<TextError>(InvalidRange);
}

public flow get_byte_slice<I ~ Index>(text: string, range: Range<I>) -> Option<string> ![] {
    return match byte_slice(text, range)? {
        Ok(value) => Some(value),
        Err(_) => None,
    };
}

public flow get_char_slice<I ~ Index>(text: string, range: Range<I>) -> Option<string> ![] {
    return match char_slice(text, range)? {
        Ok(value) => Some(value),
        Err(_) => None,
    };
}

public flow replace_range<I ~ Index>(text: string, range: Range<I>, replacement: string)
    -> string ![Error<TextError>] {
    let (start, end) = match range.text_bounds() {
        Ok(value) => value,
        Err(error) => perform Error.raise<TextError>(error),
    };
    if end > text.length { perform Error.raise<TextError>(InvalidRange); }
    let input = text.utf8;
    if !BytePattern.boundary(input, start) || !BytePattern.boundary(input, end) {
        perform Error.raise<TextError>(NotCharBoundary);
    }
    let remaining = text.length - (end - start);
    if replacement.length > usize.maximum() - remaining {
        perform Error.raise<TextError>(LengthOverflow);
    }
    let before = TextSpan { input= input, start= 0, end= start }.materialize();
    let after = TextSpan { input= input, start= end, end= text.length }.materialize();
    return before + replacement + after;
}
