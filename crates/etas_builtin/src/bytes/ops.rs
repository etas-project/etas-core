use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn len(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::Bytes(value) => Ok(BuiltinValue::Usize(value.len())),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Bytes,
            actual: other.type_tag(),
        }),
    }
}
