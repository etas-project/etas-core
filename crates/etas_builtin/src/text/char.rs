use crate::BuiltinValue;

pub fn is_ascii(value: &BuiltinValue) -> Option<bool> {
    match value {
        BuiltinValue::Char(value) => Some(value.is_ascii()),
        _ => None,
    }
}
