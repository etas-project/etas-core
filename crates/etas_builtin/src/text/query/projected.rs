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

fn contains<T: TextInput + ?Sized>(source: &T, pattern: &T) -> Result<bool, BuiltinError> {
    let length = pattern.byte_len();
    if length == 0 {
        return Ok(true);
    }
    if length > source.byte_len() {
        return Ok(false);
    }
    let pattern = super::super::search::PatternIndex::new(pattern)?;
    let mut matched = 0;
    for byte in source.chunks().flat_map(str::bytes) {
        if pattern.advance(&mut matched, byte) {
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
