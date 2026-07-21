use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn assert(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::Bool(true) => Ok(BuiltinValue::Unit),
        BuiltinValue::Bool(false) => Err(BuiltinError::AssertionFailed),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Bool,
            actual: other.type_tag(),
        }),
    }
}
