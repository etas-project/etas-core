use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn len(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 1)?;
    match &args[0] {
        BuiltinValue::Bytes(value) => Ok(len_borrowed(value)),
        other => Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Bytes,
            actual: other.type_tag(),
        }),
    }
}

pub fn len_borrowed(value: &[u8]) -> BuiltinValue {
    BuiltinValue::Usize(value.len())
}
