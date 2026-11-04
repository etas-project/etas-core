use crate::{BuiltinError, BuiltinValue, TextFailure};

use super::bounds;

pub fn char_get(text: &str, index: &BuiltinValue) -> Result<BuiltinValue, BuiltinError> {
    let Some(index) =
        bounds::CheckedPosition::from_value(index).and_then(|index| index.to_usize().ok())
    else {
        return Ok(BuiltinValue::OptionNone);
    };
    Ok(text
        .chars()
        .nth(index)
        .map(BuiltinValue::Char)
        .map(Box::new)
        .map(BuiltinValue::OptionSome)
        .unwrap_or(BuiltinValue::OptionNone))
}

pub fn checked_byte_slice(text: &str, range: &BuiltinValue) -> Result<BuiltinValue, BuiltinError> {
    let range = bounds::byte_interval(text, range).map_err(BuiltinError::Text)?;
    Ok(BuiltinValue::String(text[range].to_owned()))
}

pub fn checked_char_slice(text: &str, range: &BuiltinValue) -> Result<BuiltinValue, BuiltinError> {
    let range = bounds::scalar_interval(text, range).map_err(BuiltinError::Text)?;
    Ok(BuiltinValue::String(text[range].to_owned()))
}

pub fn optional_byte_slice(text: &str, range: &BuiltinValue) -> BuiltinValue {
    optional_slice(text, bounds::byte_interval(text, range))
}

pub fn optional_char_slice(text: &str, range: &BuiltinValue) -> BuiltinValue {
    optional_slice(text, bounds::scalar_interval(text, range))
}

fn optional_slice(text: &str, range: Result<std::ops::Range<usize>, TextFailure>) -> BuiltinValue {
    range
        .ok()
        .map(|range| BuiltinValue::String(text[range].to_owned()))
        .map(Box::new)
        .map(BuiltinValue::OptionSome)
        .unwrap_or(BuiltinValue::OptionNone)
}

pub fn is_char_boundary(text: &str, index: &BuiltinValue) -> bool {
    bounds::CheckedPosition::from_value(index)
        .and_then(|index| index.to_usize().ok())
        .is_some_and(|index| index <= text.len() && text.is_char_boundary(index))
}
