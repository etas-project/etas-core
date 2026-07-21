use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn is_some(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_option(args)
        .map(|value| BuiltinValue::Bool(matches!(value, BuiltinValue::OptionSome(_))))
}

pub fn some(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    Ok(BuiltinValue::OptionSome(Box::new(args[0].clone())))
}

pub fn is_none(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_option(args).map(|value| BuiltinValue::Bool(matches!(value, BuiltinValue::OptionNone)))
}

pub fn unwrap(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::OptionSome(value) | BuiltinValue::ResultOk(value) => Ok((**value).clone()),
        BuiltinValue::OptionNone => Err(BuiltinError::Abort {
            message: "unwrap encountered None".to_owned(),
        }),
        BuiltinValue::ResultErr(_) => Err(BuiltinError::Abort {
            message: "unwrap encountered Err".to_owned(),
        }),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Option,
            actual: other.type_tag(),
        }),
    }
}

fn expect_option(args: &[BuiltinValue]) -> Result<&BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        value @ (BuiltinValue::OptionSome(_) | BuiltinValue::OptionNone) => Ok(value),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Option,
            actual: other.type_tag(),
        }),
    }
}
