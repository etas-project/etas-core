use super::BuiltinValue;

pub trait BuiltinValueAdapter {
    type Value;
    type Error;

    fn into_builtin(value: Self::Value) -> Result<BuiltinValue, Self::Error>;
    fn from_builtin(value: BuiltinValue) -> Result<Self::Value, Self::Error>;
}
