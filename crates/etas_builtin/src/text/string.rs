use super::query::TextQuery;
use super::transform::TextTransform;
use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn trim(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_transform(args, TextTransform::Trim)
}

pub fn len(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_query(args, TextQuery::Len)
}

pub fn lowercase(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_transform(args, TextTransform::Lowercase)
}

pub fn uppercase(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_transform(args, TextTransform::Uppercase)
}

pub fn contains(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_query(args, TextQuery::Contains)
}

pub fn starts_with(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_query(args, TextQuery::StartsWith)
}

pub fn ends_with(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_query(args, TextQuery::EndsWith)
}

pub fn lines(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_transform(args, TextTransform::Lines)
}

pub fn split(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_transform(args, TextTransform::Split)
}

pub fn join(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let BuiltinValue::Array(parts) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Array,
            actual: args[0].type_tag(),
        });
    };
    let BuiltinValue::String(separator) = &args[1] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: args[1].type_tag(),
        });
    };
    let mut rendered = Vec::with_capacity(parts.len());
    for part in parts {
        let BuiltinValue::String(value) = part else {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::String,
                actual: part.type_tag(),
            });
        };
        rendered.push(value.as_str());
    }
    Ok(BuiltinValue::String(rendered.join(separator)))
}

pub fn to_string_i32(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_integer_to_string(args, |value| (value as i32).to_string())
}

pub fn to_string_usize(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    unary_integer_to_string(args, |value| (value as usize).to_string())
}

pub fn parse_i32(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    text_query(args, TextQuery::ParseI32)
}

fn text_transform(
    args: &[BuiltinValue],
    transform: TextTransform,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, transform.arity())?;
    let mut borrowed = [""; 2];
    for (slot, arg) in borrowed.iter_mut().zip(args) {
        let BuiltinValue::String(value) = arg else {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::String,
                actual: arg.type_tag(),
            });
        };
        *slot = value;
    }
    transform
        .evaluate(&borrowed[..args.len()])
        .map(|value| value.into_owned())
}

fn text_query(args: &[BuiltinValue], query: TextQuery) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, query.arity())?;
    let mut borrowed = [""; 2];
    for (slot, arg) in borrowed.iter_mut().zip(args) {
        let BuiltinValue::String(value) = arg else {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::String,
                actual: arg.type_tag(),
            });
        };
        *slot = value;
    }
    query.evaluate(&borrowed[..args.len()])
}

fn unary_integer_to_string(
    args: &[BuiltinValue],
    render: impl FnOnce(i64) -> String,
) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    let value = match &args[0] {
        BuiltinValue::I32(value) => i64::from(*value),
        BuiltinValue::I64(value) => *value,
        BuiltinValue::Usize(value) => *value as i64,
        other => {
            return Err(BuiltinError::TypeMismatch {
                expected: BuiltinTypeTag::I64,
                actual: other.type_tag(),
            });
        }
    };
    Ok(BuiltinValue::String(render(value)))
}
