use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn is_ok(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_result(args).map(|value| BuiltinValue::Bool(matches!(value, BuiltinValue::ResultOk(_))))
}

pub fn ok(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    Ok(BuiltinValue::ResultOk(Box::new(args[0].clone())))
}

pub fn is_err(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_result(args).map(|value| BuiltinValue::Bool(matches!(value, BuiltinValue::ResultErr(_))))
}

pub fn err(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    Ok(BuiltinValue::ResultErr(Box::new(args[0].clone())))
}

fn expect_result(args: &[BuiltinValue]) -> Result<&BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        value @ (BuiltinValue::ResultOk(_) | BuiltinValue::ResultErr(_)) => Ok(value),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Result,
            actual: other.type_tag(),
        }),
    }
}
