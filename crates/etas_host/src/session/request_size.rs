use super::*;
use crate::{HostError, StorageLimits};

impl SessionRequest {
    pub(crate) fn storage_size(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        bounded(
            match &self.operation {
                SessionOperation::Resolve { config } => config.id.len(),
                SessionOperation::Append { message } => message.storage_size(limits)?,
                SessionOperation::Load {
                    session, cursor, ..
                } => session
                    .id
                    .len()
                    .checked_add(cursor.as_ref().map_or(0, |c| c.opaque.len()))
                    .ok_or_else(write::limit_error)?,
            },
            limits,
        )
    }
}
impl SessionWriteRequest {
    pub(crate) fn storage_size(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        bounded(
            match &self.operation {
                SessionWriteOperation::PublishContext(publication) => {
                    publication.retained_size(limits)?
                }
                SessionWriteOperation::Resolve { key, config } => config
                    .id
                    .len()
                    .checked_add(key.as_str().len())
                    .ok_or_else(write::limit_error)?,
                SessionWriteOperation::Append { key, message } => message
                    .storage_size(limits)?
                    .checked_add(key.as_str().len())
                    .ok_or_else(write::limit_error)?,
                SessionWriteOperation::Reconcile { session, operation }
                | SessionWriteOperation::ReconcileContext { session, operation } => {
                    operation.validate()?;
                    session
                        .id
                        .len()
                        .checked_add(operation.key.as_str().len())
                        .and_then(|n| n.checked_add(operation.request_fingerprint.len()))
                        .ok_or_else(write::limit_error)?
                }
            },
            limits,
        )
    }
}
fn bounded(bytes: usize, limits: &StorageLimits) -> Result<usize, HostError> {
    if bytes > limits.max_value_bytes {
        return Err(write::limit_error());
    }
    Ok(bytes)
}
