use crate::BuiltinValue;

pub fn stringify_scalar(value: &BuiltinValue) -> Option<String> {
    match value {
        BuiltinValue::Unit => Some("null".to_owned()),
        BuiltinValue::Bool(value) => Some(value.to_string()),
        BuiltinValue::String(value) => Some(format!("{value:?}")),
        BuiltinValue::I64(value) => Some(value.to_string()),
        BuiltinValue::U64(value) => Some(value.to_string()),
        BuiltinValue::F64(value) if value.is_finite() => Some(value.to_string()),
        _ => None,
    }
}
