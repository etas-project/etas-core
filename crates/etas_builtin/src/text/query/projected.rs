use super::{BuiltinError, BuiltinValue, TextInput, TextQuery};

pub(super) fn evaluate<T: TextInput + ?Sized>(
    query: TextQuery,
    args: &[&T],
) -> Result<BuiltinValue, BuiltinError> {
    Ok(match query {
        TextQuery::Len => BuiltinValue::Usize(args[0].byte_len()),
        TextQuery::CharCount => {
            BuiltinValue::Usize(args[0].chunks().map(|chunk| chunk.chars().count()).sum())
        }
        TextQuery::IsEmpty => BuiltinValue::Bool(args[0].byte_len() == 0),
        TextQuery::IsAscii => BuiltinValue::Bool(args[0].chunks().all(str::is_ascii)),
        TextQuery::StartsWith => BuiltinValue::Bool(starts_with(args[0], args[1])),
        TextQuery::EndsWith => BuiltinValue::Bool(ends_with(args[0], args[1])),
        TextQuery::Contains => BuiltinValue::Bool(contains(args[0], args[1])?),
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
