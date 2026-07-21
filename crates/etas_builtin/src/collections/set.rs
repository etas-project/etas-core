use crate::BuiltinValue;

pub fn len(value: &BuiltinValue) -> Option<usize> {
    match value {
        BuiltinValue::Set(value) => Some(value.len()),
        _ => None,
    }
}
