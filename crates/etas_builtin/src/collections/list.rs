use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn len(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::Array(value) => Ok(BuiltinValue::Usize(value.len())),
        BuiltinValue::List(value) => Ok(BuiltinValue::Usize(value.len())),
        BuiltinValue::Slice(value) => Ok(BuiltinValue::Usize(value.len())),
        BuiltinValue::Map(value) => Ok(BuiltinValue::Usize(value.len())),
        BuiltinValue::String(value) => Ok(BuiltinValue::Usize(value.chars().count())),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Array,
            actual: other.type_tag(),
        }),
    }
}

pub fn len_from_count(count: usize) -> BuiltinValue {
    BuiltinValue::Usize(count)
}

pub fn is_empty(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::Array(value) => Ok(BuiltinValue::Bool(value.is_empty())),
        BuiltinValue::List(value) => Ok(BuiltinValue::Bool(value.is_empty())),
        BuiltinValue::Slice(value) => Ok(BuiltinValue::Bool(value.is_empty())),
        BuiltinValue::Map(value) => Ok(BuiltinValue::Bool(value.is_empty())),
        BuiltinValue::String(value) => Ok(BuiltinValue::Bool(value.is_empty())),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Array,
            actual: other.type_tag(),
        }),
    }
}

pub fn is_empty_from_count(count: usize) -> BuiltinValue {
    BuiltinValue::Bool(count == 0)
}
