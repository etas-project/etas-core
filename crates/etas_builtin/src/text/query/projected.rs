use super::{BuiltinError, BuiltinValue, TextInput, TextQuery};

pub(super) fn evaluate<T: TextInput + ?Sized>(
    query: TextQuery,
    args: &[&T],
) -> Result<BuiltinValue, BuiltinError> {
    Ok(match query {
        TextQuery::Len => {
            BuiltinValue::Usize(args[0].chunks().try_fold(0usize, |count, chunk| {
                count
                    .checked_add(chunk.chars().count())
                    .ok_or(BuiltinError::NumericOverflow)
            })?)
        }
        TextQuery::StartsWith => BuiltinValue::Bool(starts_with(args[0], args[1])),
        TextQuery::EndsWith => BuiltinValue::Bool(ends_with(args[0], args[1])),
        TextQuery::Contains => BuiltinValue::Bool(contains(args[0], args[1])?),
        TextQuery::ParseI32 => match parse_i32(args[0]) {
            Ok(value) => BuiltinValue::ResultOk(Box::new(BuiltinValue::I32(value))),
            Err(error) => {
                BuiltinValue::ResultErr(Box::new(BuiltinValue::String(error.to_string())))
            }
        },
    })
}

fn starts_with<T: TextInput + ?Sized>(source: &T, pattern: &T) -> bool {
    let length = pattern.byte_len();
    length == 0
        || (length <= source.byte_len()
            && source
                .chunks()
                .flat_map(str::bytes)
                .take(length)
                .eq(pattern.chunks().flat_map(str::bytes)))
}

fn ends_with<T: TextInput + ?Sized>(source: &T, pattern: &T) -> bool {
    let length = pattern.byte_len();
    length == 0
        || (length <= source.byte_len()
            && source
                .chunks()
                .rev()
                .flat_map(|chunk| chunk.bytes().rev())
                .take(length)
                .eq(pattern.chunks().rev().flat_map(|chunk| chunk.bytes().rev())))
}

enum PatternBytes<'a> {
    Flat(&'a [u8]),
    Chunks(Vec<PatternChunk<'a>>),
}

struct PatternChunk<'a> {
    end: usize,
    bytes: &'a [u8],
}

impl PatternBytes<'_> {
    fn at(&self, index: usize) -> u8 {
        match self {
            Self::Flat(bytes) => bytes[index],
            Self::Chunks(chunks) => {
                let slot = chunks.partition_point(|chunk| chunk.end <= index);
                let chunk = &chunks[slot];
                chunk.bytes[index - (chunk.end - chunk.bytes.len())]
            }
        }
    }
}

fn contains<T: TextInput + ?Sized>(source: &T, pattern: &T) -> Result<bool, BuiltinError> {
    let length = pattern.byte_len();
    if length == 0 {
        return Ok(true);
    }
    if length > source.byte_len() {
        return Ok(false);
    }
    // KMP stores only prefix lengths and a per-chunk index, not a byte-sized
    // pointer table or a flattened payload. Chunked pattern lookup is logarithmic
    // in its chunk count; flat patterns retain direct byte indexing.
    let bytes = if let Some(text) = pattern.contiguous() {
        PatternBytes::Flat(text.as_bytes())
    } else {
        let chunks = pattern.chunks();
        let mut indexed = Vec::new();
        indexed
            .try_reserve_exact(chunks.size_hint().0)
            .map_err(|_| BuiltinError::NumericOverflow)?;
        let mut end = 0usize;
        for chunk in chunks.filter(|chunk| !chunk.is_empty()) {
            end = end
                .checked_add(chunk.len())
                .ok_or(BuiltinError::NumericOverflow)?;
            indexed.push(PatternChunk {
                end,
                bytes: chunk.as_bytes(),
            });
        }
        if end != length {
            return Err(BuiltinError::Abort {
                message: "invalid text projection length".into(),
            });
        }
        PatternBytes::Chunks(indexed)
    };
    let mut prefix = Vec::new();
    prefix
        .try_reserve_exact(length)
        .map_err(|_| BuiltinError::NumericOverflow)?;
    prefix.resize(length, 0usize);
    let mut matched = 0;
    for index in 1..length {
        while matched > 0 && bytes.at(matched) != bytes.at(index) {
            matched = prefix[matched - 1];
        }
        if bytes.at(matched) == bytes.at(index) {
            matched += 1;
        }
        prefix[index] = matched;
    }
    matched = 0;
    for byte in source.chunks().flat_map(str::bytes) {
        while matched > 0 && bytes.at(matched) != byte {
            matched = prefix[matched - 1];
        }
        if bytes.at(matched) == byte {
            matched += 1;
        }
        if matched == length {
            return Ok(true);
        }
    }
    Ok(false)
}

fn parse_i32<T: TextInput + ?Sized>(text: &T) -> Result<i32, std::num::ParseIntError> {
    let mut chars = text.chunks().flat_map(str::chars).peekable();
    let digits = i32::MAX.ilog10() as usize + 1;
    let mut token = String::with_capacity(digits + 1 + 4);
    if let Some(sign @ ('+' | '-')) = chars.peek().copied() {
        chars.next();
        token.push(sign);
    }
    let mut significant = 0;
    let mut zero = false;
    for ch in chars {
        // Skip arbitrarily many leading zeroes, not input bytes in general.
        // The bounded significant prefix keeps the standard parser's exact
        // value/error semantics, including overflow before a later bad digit.
        if ch == '0' && significant == 0 {
            zero = true;
            continue;
        }
        token.push(ch);
        if !ch.is_ascii_digit() {
            return token.parse();
        }
        significant += 1;
        if significant >= digits {
            token.parse::<i32>()?;
        }
    }
    if significant == 0 && zero {
        token.push('0');
    }
    token.parse()
}
