use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn abort(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::String(message) => Err(BuiltinError::Abort {
            message: message.clone(),
        }),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::String,
            actual: other.type_tag(),
        }),
    }
}
