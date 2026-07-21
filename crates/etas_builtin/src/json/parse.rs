use crate::BuiltinError;

pub fn parse_json_descriptor_only() -> Result<(), BuiltinError> {
    Err(BuiltinError::InvalidJson)
}
