use etas_std::{StdIntrinsicId, intrinsic::pure};

use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, TextFailure, error::expect_arity};

use super::{bounds, format, parse, ranges, search, slice, split, unicode};

pub fn is_text_intrinsic(intrinsic: StdIntrinsicId) -> bool {
    matches!(
        intrinsic.0,
        pure::TEXT_TRIM
            | pure::TEXT_CONTAINS
            | pure::TEXT_LOWERCASE
            | pure::TEXT_UPPERCASE
            | pure::TEXT_STARTS_WITH
            | pure::TEXT_ENDS_WITH
            | pure::TEXT_LEN
            | pure::TEXT_SPLIT
            | pure::TEXT_JOIN
            | pure::TEXT_CHAR_COUNT
            | pure::TEXT_IS_EMPTY
            | pure::TEXT_IS_ASCII
            | pure::TEXT_CHARS
            | pure::TEXT_CHAR_INDICES
            | pure::TEXT_LINES
            | pure::TEXT_FIND
            | pure::TEXT_RFIND
            | pure::TEXT_FIND_FROM
            | pure::TEXT_FIND_ALL
            | pure::TEXT_COUNT
            | pure::TEXT_SPLIT_ITER
            | pure::TEXT_SPLIT_ONCE
            | pure::TEXT_RSPLIT_ONCE
            | pure::TEXT_SPLITN
            | pure::TEXT_SPLIT_WHITESPACE
            | pure::TEXT_CHAR_GET
            | pure::TEXT_BYTE_SLICE
            | pure::TEXT_CHAR_SLICE
            | pure::TEXT_GET_BYTE_SLICE
            | pure::TEXT_GET_CHAR_SLICE
            | pure::TEXT_IS_CHAR_BOUNDARY
            | pure::TEXT_TRIM_START
            | pure::TEXT_TRIM_END
            | pure::TEXT_STRIP_PREFIX
            | pure::TEXT_STRIP_SUFFIX
            | pure::TEXT_ASCII_LOWERCASE
            | pure::TEXT_ASCII_UPPERCASE
            | pure::TEXT_EQ_IGNORE_ASCII_CASE
            | pure::TEXT_REPLACE
            | pure::TEXT_REPLACE_N
            | pure::TEXT_REPLACE_RANGE
            | pure::TEXT_CONCAT
            | pure::TEXT_REPEAT
            | pure::TEXT_PAD_START
            | pure::TEXT_PAD_END
            | pure::TEXT_REVERSE_CHARS
            | pure::TEXT_PARSE_I8..=pure::TEXT_FORMAT
            | pure::TEXT_GRAPHEMES..=pure::TEXT_CASEFOLD
            | pure::CHAR_IS_WHITESPACE..=pure::CHAR_FROM_U32
    )
}

pub fn call_text_intrinsic(
    intrinsic: StdIntrinsicId,
    args: &[BuiltinValue],
) -> Result<BuiltinValue, BuiltinError> {
    match intrinsic.0 {
        pure::TEXT_LEN => unary_text(args, |text| BuiltinValue::Usize(text.len())),
        pure::TEXT_CHAR_COUNT => unary_text(args, |text| BuiltinValue::Usize(text.chars().count())),
        pure::TEXT_IS_EMPTY => unary_text(args, |text| BuiltinValue::Bool(text.is_empty())),
        pure::TEXT_IS_ASCII => unary_text(args, |text| BuiltinValue::Bool(text.is_ascii())),
        pure::TEXT_CONTAINS => binary_text(args, |source, pattern| {
            BuiltinValue::Bool(source.contains(pattern))
        }),
        pure::TEXT_STARTS_WITH => binary_text(args, |source, pattern| {
            BuiltinValue::Bool(source.starts_with(pattern))
        }),
        pure::TEXT_ENDS_WITH => binary_text(args, |source, pattern| {
            BuiltinValue::Bool(source.ends_with(pattern))
        }),
        pure::TEXT_FIND => binary_text(args, |source, pattern| {
            option_usize(search::find(source, pattern))
        }),
        pure::TEXT_RFIND => binary_text(args, |source, pattern| {
            option_usize(search::rfind(source, pattern))
        }),
        pure::TEXT_COUNT => binary_text(args, |source, pattern| {
            BuiltinValue::Usize(search::count(source, pattern))
        }),
        pure::TEXT_FIND_FROM => find_from(args),
        pure::TEXT_SPLIT => binary_text(args, |source, separator| {
            BuiltinValue::Array(split::split(source, separator))
        }),
        pure::TEXT_SPLIT_ONCE => binary_text(args, |source, separator| {
            option_value(split::split_once(source, separator))
        }),
        pure::TEXT_RSPLIT_ONCE => binary_text(args, |source, separator| {
            option_value(split::rsplit_once(source, separator))
        }),
        pure::TEXT_SPLITN => splitn(args),
        pure::TEXT_CHAR_GET => text_and_value(args, slice::char_get)?,
        pure::TEXT_BYTE_SLICE => text_and_value(args, slice::checked_byte_slice)?,
        pure::TEXT_CHAR_SLICE => text_and_value(args, slice::checked_char_slice)?,
        pure::TEXT_GET_BYTE_SLICE => text_and_value(args, |text, value| {
            Ok(slice::optional_byte_slice(text, value))
        })?,
        pure::TEXT_GET_CHAR_SLICE => text_and_value(args, |text, value| {
            Ok(slice::optional_char_slice(text, value))
        })?,
        pure::TEXT_IS_CHAR_BOUNDARY => text_and_value(args, |text, value| {
            Ok(BuiltinValue::Bool(slice::is_char_boundary(text, value)))
        })?,
        pure::TEXT_TRIM => trim(args, ranges::TextTrimKind::Both),
        pure::TEXT_TRIM_START => trim(args, ranges::TextTrimKind::Start),
        pure::TEXT_TRIM_END => trim(args, ranges::TextTrimKind::End),
        pure::TEXT_LOWERCASE => case(args, super::case::CaseMapping::Lowercase),
        pure::TEXT_UPPERCASE => case(args, super::case::CaseMapping::Uppercase),
        pure::TEXT_ASCII_LOWERCASE => {
            unary_text(args, |text| BuiltinValue::String(text.to_ascii_lowercase()))
        }
        pure::TEXT_ASCII_UPPERCASE => {
            unary_text(args, |text| BuiltinValue::String(text.to_ascii_uppercase()))
        }
        pure::TEXT_STRIP_PREFIX => binary_text(args, |source, pattern| {
            option_string(source.strip_prefix(pattern))
        }),
        pure::TEXT_STRIP_SUFFIX => binary_text(args, |source, pattern| {
            option_string(source.strip_suffix(pattern))
        }),
        pure::TEXT_EQ_IGNORE_ASCII_CASE => binary_text(args, |source, other| {
            BuiltinValue::Bool(source.eq_ignore_ascii_case(other))
        }),
        pure::TEXT_REPLACE => replace(args, None),
        pure::TEXT_REPLACE_N => replace_n(args),
        pure::TEXT_REPLACE_RANGE => replace_range(args),
        pure::TEXT_CONCAT => assemble(args, ""),
        pure::TEXT_JOIN => join(args),
        pure::TEXT_REPEAT => repeat(args),
        pure::TEXT_PAD_START => pad(args, true),
        pure::TEXT_PAD_END => pad(args, false),
        pure::TEXT_REVERSE_CHARS => unary_text_fallible(args, reverse_chars),
        pure::TEXT_PARSE_I8 => parse_text(args, parse::parse_i8),
        pure::TEXT_PARSE_I16 => parse_text(args, parse::parse_i16),
        pure::TEXT_PARSE_I32 => parse_text(args, parse::parse_i32),
        pure::TEXT_PARSE_I64 => parse_text(args, parse::parse_i64),
        pure::TEXT_PARSE_I128 => parse_text(args, parse::parse_i128),
        pure::TEXT_PARSE_ISIZE => parse_text(args, parse::parse_isize),
        pure::TEXT_PARSE_U8 => parse_text(args, parse::parse_u8),
        pure::TEXT_PARSE_U16 => parse_text(args, parse::parse_u16),
        pure::TEXT_PARSE_U32 => parse_text(args, parse::parse_u32),
        pure::TEXT_PARSE_U64 => parse_text(args, parse::parse_u64),
        pure::TEXT_PARSE_U128 => parse_text(args, parse::parse_u128),
        pure::TEXT_PARSE_USIZE => parse_text(args, parse::parse_usize),
        pure::TEXT_PARSE_F32 => parse_text(args, parse::parse_f32),
        pure::TEXT_PARSE_F64 => parse_text(args, parse::parse_f64),
        pure::TEXT_PARSE_BOOL => parse_text(args, parse::parse_bool),
        pure::TEXT_FROM_I8..=pure::TEXT_FROM_CHAR => render(args),
        pure::TEXT_FORMAT => checked_format(args),
        pure::TEXT_GRAPHEME_COUNT => {
            unary_text(args, |text| BuiltinValue::Usize(grapheme_count(text)))
        }
        pure::TEXT_GRAPHEME_SLICE => grapheme_slice(args),
        pure::TEXT_REVERSE_GRAPHEMES => unary_text_fallible(args, reverse_graphemes),
        pure::TEXT_NORMALIZE => normalize(args, false),
        pure::TEXT_IS_NORMALIZED => normalize(args, true),
        pure::TEXT_CASEFOLD => unary_text_fallible(args, |source| {
            super::unicode::casefold(source).map(BuiltinValue::String)
        }),
        pure::CHAR_IS_WHITESPACE => unary_char(args, |value| {
            BuiltinValue::Bool(unicode::is_whitespace(value))
        }),
        pure::CHAR_IS_ALPHABETIC => unary_char(args, |value| {
            BuiltinValue::Bool(unicode::is_alphabetic(value))
        }),
        pure::CHAR_IS_ALPHANUMERIC => unary_char(args, |value| {
            BuiltinValue::Bool(unicode::is_alphanumeric(value))
        }),
        pure::CHAR_IS_CONTROL => {
            unary_char(args, |value| BuiltinValue::Bool(unicode::is_control(value)))
        }
        pure::CHAR_IS_ASCII_DIGIT => {
            unary_char(args, |value| BuiltinValue::Bool(value.is_ascii_digit()))
        }
        pure::CHAR_TO_U32 => unary_char(args, |value| BuiltinValue::U32(value as u32)),
        pure::CHAR_FROM_U32 => from_u32(args),
        _ => Err(BuiltinError::UnsupportedIntrinsic { intrinsic }),
    }
}

fn trim(args: &[BuiltinValue], kind: ranges::TextTrimKind) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let text = expect_text(&args[0])?;
    let range = ranges::trim_range_kind(text, kind);
    Ok(BuiltinValue::String(text[range].to_owned()))
}

fn case(
    args: &[BuiltinValue],
    mapping: super::case::CaseMapping,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let text = expect_text(&args[0])?;
    let output = match super::case::map_case(text, mapping)? {
        super::case::CaseOutput::Unchanged => text.to_owned(),
        super::case::CaseOutput::Changed(output) => output,
    };
    Ok(BuiltinValue::String(output))
}

fn unary_text(
    args: &[BuiltinValue],
    operation: impl FnOnce(&str) -> BuiltinValue,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    Ok(operation(expect_text(&args[0])?))
}

fn unary_text_fallible(
    args: &[BuiltinValue],
    operation: impl FnOnce(&str) -> Result<BuiltinValue, BuiltinError>,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    operation(expect_text(&args[0])?)
}

fn binary_text(
    args: &[BuiltinValue],
    operation: impl FnOnce(&str, &str) -> BuiltinValue,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    Ok(operation(expect_text(&args[0])?, expect_text(&args[1])?))
}

fn text_and_value(
    args: &[BuiltinValue],
    operation: impl FnOnce(&str, &BuiltinValue) -> Result<BuiltinValue, BuiltinError>,
) -> Result<Result<BuiltinValue, BuiltinError>, BuiltinError> {
    expect_arity(args, 2)?;
    Ok(operation(expect_text(&args[0])?, &args[1]))
}

fn expect_text(value: &BuiltinValue) -> Result<&str, BuiltinError> {
    let BuiltinValue::String(value) = value else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: value.type_tag(),
        });
    };
    Ok(value)
}

fn option_value(value: Option<BuiltinValue>) -> BuiltinValue {
    value
        .map(Box::new)
        .map(BuiltinValue::OptionSome)
        .unwrap_or(BuiltinValue::OptionNone)
}

fn option_string(value: Option<&str>) -> BuiltinValue {
    option_value(value.map(|value| BuiltinValue::String(value.to_owned())))
}

fn option_usize(value: Option<usize>) -> BuiltinValue {
    option_value(value.map(BuiltinValue::Usize))
}

fn find_from(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 3)?;
    let start = bounds::position(&args[2]).map_err(BuiltinError::Text)?;
    search::find_from(expect_text(&args[0])?, expect_text(&args[1])?, start)
        .map(option_usize)
        .map_err(BuiltinError::Text)
}

fn splitn(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 3)?;
    let BuiltinValue::Usize(count) = args[2] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Usize,
            actual: args[2].type_tag(),
        });
    };
    Ok(BuiltinValue::Array(split::splitn(
        expect_text(&args[0])?,
        expect_text(&args[1])?,
        count,
    )))
}

fn replace(args: &[BuiltinValue], limit: Option<usize>) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 3)?;
    let source = expect_text(&args[0])?;
    let pattern = expect_text(&args[1])?;
    let replacement = expect_text(&args[2])?;
    let matches = search::matches(source, pattern).take(limit.unwrap_or(usize::MAX));
    let match_count = matches.clone().count();
    if match_count == 0 {
        return Ok(BuiltinValue::String(source.to_owned()));
    }
    let removed = matches.clone().map(|range| range.len()).sum::<usize>();
    let bytes = source
        .len()
        .checked_sub(removed)
        .and_then(|bytes| bytes.checked_add(replacement.len().checked_mul(match_count)?))
        .ok_or(BuiltinError::Text(TextFailure::LengthOverflow))?;
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    let mut copied = 0;
    for range in matches {
        output.push_str(&source[copied..range.start]);
        output.push_str(replacement);
        copied = range.end;
    }
    output.push_str(&source[copied..]);
    Ok(BuiltinValue::String(output))
}

fn replace_n(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 4)?;
    let BuiltinValue::Usize(limit) = args[3] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Usize,
            actual: args[3].type_tag(),
        });
    };
    replace(&args[..3], Some(limit))
}

fn replace_range(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 3)?;
    let source = expect_text(&args[0])?;
    let range = bounds::byte_interval(source, &args[1]).map_err(BuiltinError::Text)?;
    let replacement = expect_text(&args[2])?;
    let bytes = source
        .len()
        .checked_sub(range.len())
        .and_then(|bytes| bytes.checked_add(replacement.len()))
        .ok_or(BuiltinError::Text(TextFailure::LengthOverflow))?;
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    output.push_str(&source[..range.start]);
    output.push_str(replacement);
    output.push_str(&source[range.end..]);
    Ok(BuiltinValue::String(output))
}

fn expect_parts(value: &BuiltinValue) -> Result<Vec<&str>, BuiltinError> {
    let BuiltinValue::Array(parts) = value else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Array,
            actual: value.type_tag(),
        });
    };
    parts.iter().map(expect_text).collect()
}

fn assemble(args: &[BuiltinValue], separator: &str) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let parts = expect_parts(&args[0])?;
    super::join::join_projected(parts.iter().copied().map(Ok::<_, ()>), separator)
        .map(BuiltinValue::String)
        .map_err(|error| match error {
            super::join::JoinError::Projection(()) => unreachable!(),
            super::join::JoinError::LengthOverflow => {
                BuiltinError::Text(TextFailure::LengthOverflow)
            }
            super::join::JoinError::AllocationFailed => BuiltinError::ResourceExhausted,
        })
}

fn join(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let separator = expect_text(&args[1])?;
    assemble(&args[..1], separator)
}

fn repeat(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let source = expect_text(&args[0])?;
    let BuiltinValue::Usize(count) = args[1] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Usize,
            actual: args[1].type_tag(),
        });
    };
    if source.is_empty() || count == 0 {
        return Ok(BuiltinValue::String(String::new()));
    }
    let bytes = source
        .len()
        .checked_mul(count)
        .ok_or(BuiltinError::Text(TextFailure::LengthOverflow))?;
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    for _ in 0..count {
        output.push_str(source);
    }
    Ok(BuiltinValue::String(output))
}

fn pad(args: &[BuiltinValue], start: bool) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 3)?;
    let source = expect_text(&args[0])?;
    let BuiltinValue::Usize(width) = args[1] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Usize,
            actual: args[1].type_tag(),
        });
    };
    let BuiltinValue::Char(fill) = args[2] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Char,
            actual: args[2].type_tag(),
        });
    };
    let count = source.chars().count();
    if count >= width {
        return Ok(BuiltinValue::String(source.to_owned()));
    }
    let padding = width - count;
    let bytes = source
        .len()
        .checked_add(
            fill.len_utf8()
                .checked_mul(padding)
                .ok_or(BuiltinError::Text(TextFailure::LengthOverflow))?,
        )
        .ok_or(BuiltinError::Text(TextFailure::LengthOverflow))?;
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    if !start {
        output.push_str(source);
    }
    output.extend(std::iter::repeat_n(fill, padding));
    if start {
        output.push_str(source);
    }
    Ok(BuiltinValue::String(output))
}

fn parse_text(
    args: &[BuiltinValue],
    parser: fn(&str) -> Result<BuiltinValue, TextFailure>,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    Ok(parse::result(parser(expect_text(&args[0])?)))
}

fn render(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    parse::render(&args[0])
        .map(BuiltinValue::String)
        .ok_or(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[0].type_tag(),
        })
}

fn checked_format(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let template = expect_text(&args[0])?;
    let parts = expect_parts(&args[1])?
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    Ok(parse::result(
        format::format(template, &parts)?.map(BuiltinValue::String),
    ))
}

fn unary_char(
    args: &[BuiltinValue],
    operation: impl FnOnce(char) -> BuiltinValue,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let BuiltinValue::Char(value) = args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Char,
            actual: args[0].type_tag(),
        });
    };
    Ok(operation(value))
}

fn from_u32(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let BuiltinValue::U32(value) = args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::U32,
            actual: args[0].type_tag(),
        });
    };
    Ok(option_value(char::from_u32(value).map(BuiltinValue::Char)))
}

fn grapheme_count(source: &str) -> usize {
    let mut count = 0;
    let mut offset = 0;
    while offset < source.len() {
        offset = super::unicode::next_grapheme_boundary(source, offset);
        count += 1;
    }
    count
}

fn reverse_chars(source: &str) -> Result<BuiltinValue, BuiltinError> {
    let mut output = String::new();
    output
        .try_reserve_exact(source.len())
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    output.extend(source.chars().rev());
    Ok(BuiltinValue::String(output))
}

fn reverse_graphemes(source: &str) -> Result<BuiltinValue, BuiltinError> {
    let count = grapheme_count(source);
    let mut boundaries = Vec::new();
    boundaries
        .try_reserve_exact(count + 1)
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    boundaries.push(0);
    let mut byte = 0;
    while byte < source.len() {
        byte = super::unicode::next_grapheme_boundary(source, byte);
        boundaries.push(byte);
    }
    let mut output = String::new();
    output
        .try_reserve_exact(source.len())
        .map_err(|_| BuiltinError::ResourceExhausted)?;
    for pair in boundaries.windows(2).rev() {
        output.push_str(&source[pair[0]..pair[1]]);
    }
    Ok(BuiltinValue::String(output))
}

fn grapheme_slice(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let source = expect_text(&args[0])?;
    let range = grapheme_byte_interval(source, &args[1]).map_err(BuiltinError::Text)?;
    Ok(BuiltinValue::String(source[range].to_owned()))
}

fn normalize(args: &[BuiltinValue], query: bool) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let source = expect_text(&args[0])?;
    let form = super::unicode::NormalizationForm::from_builtin(&args[1])?;
    if query {
        super::unicode::is_normalized(source, form).map(BuiltinValue::Bool)
    } else {
        super::unicode::normalize(source, form).map(BuiltinValue::String)
    }
}

fn grapheme_byte_interval(
    source: &str,
    value: &BuiltinValue,
) -> Result<std::ops::Range<usize>, TextFailure> {
    let count = grapheme_count(source);
    let range = bounds::interval(value, count)?;
    let mut ordinal = 0;
    let mut byte = 0;
    let mut start = (range.start == 0).then_some(0);
    let mut end = (range.end == 0).then_some(0);
    while byte < source.len() && end.is_none() {
        byte = super::unicode::next_grapheme_boundary(source, byte);
        ordinal += 1;
        if ordinal == range.start {
            start = Some(byte);
        }
        if ordinal == range.end {
            end = Some(byte);
        }
    }
    Ok(start.ok_or(TextFailure::InvalidRange)?..end.ok_or(TextFailure::InvalidRange)?)
}
