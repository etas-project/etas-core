use crate::{BuiltinError, BuiltinTypeTag, BuiltinValue, error::expect_arity};

pub fn len(value: &BuiltinValue) -> Option<usize> {
    match value {
        BuiltinValue::Map(value) => Some(value.len()),
        _ => None,
    }
}

pub fn contains_key(args: &[BuiltinValue]) -> Result<BuiltinValue, BuiltinError> {
    expect_arity(args, 2)?;
    let BuiltinValue::Map(entries) = &args[0] else {
        return Err(BuiltinError::TypeMismatch {
            expected: BuiltinTypeTag::Map,
            actual: args[0].type_tag(),
        });
    };
    let key = &args[1];
    Ok(BuiltinValue::Bool(
        entries.iter().any(|(candidate, _)| candidate == key),
    ))
}
