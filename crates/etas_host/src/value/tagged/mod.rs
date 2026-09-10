use crate::{HostError, HostErrorCode};
mod decode;
mod encode;
mod record;
#[cfg(test)]
mod tests;
pub(crate) use decode::decode as decode_with_limits;
pub(crate) use encode::encode as encode_with_limits;
pub(crate) use encode::encode_to;
pub(crate) use record::{RecordValueRef, encode_record_to};
fn schema(message: impl Into<String>) -> HostError {
    HostError::new(HostErrorCode::SchemaMismatch, message)
}
fn limit() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "stored value exceeds codec limits",
    )
}
