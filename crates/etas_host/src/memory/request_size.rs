use super::*;
use crate::{HostError, StorageLimits};

impl MemoryRequest {
    pub(crate) fn storage_size(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        let mut size = store_size(&self.store, limits)?;
        let payload = match &self.operation {
            MemoryOperation::Get { key } => limits.value_size(key)?,
            MemoryOperation::Put {
                key,
                value,
                condition,
            } => limits
                .value_size(key)?
                .checked_add(limits.value_size(value)?)
                .and_then(|n| n.checked_add(condition_size(condition)))
                .ok_or_else(write::limit_error)?,
            MemoryOperation::Delete { key, condition } => limits
                .value_size(key)?
                .checked_add(condition_size(condition))
                .ok_or_else(write::limit_error)?,
            MemoryOperation::Scan { cursor, .. } => cursor.as_ref().map_or(0, |c| c.opaque.len()),
            MemoryOperation::Query { query, .. } => {
                if query.order_by.len() > limits.max_nodes {
                    return Err(write::limit_error());
                }
                let mut bytes = query
                    .predicate
                    .as_ref()
                    .map(|v| limits.value_size(v))
                    .transpose()?
                    .unwrap_or(0);
                for order in &query.order_by {
                    add(&mut bytes, path_size(&order.field_path, limits)?, limits)?;
                }
                bytes
            }
            MemoryOperation::VectorSearch {
                embedding, filter, ..
            } => {
                if embedding.len() > limits.max_nodes {
                    return Err(write::limit_error());
                }
                let filter_bytes = filter
                    .as_ref()
                    .map(|v| limits.value_size(v))
                    .transpose()?
                    .unwrap_or(0);
                embedding
                    .len()
                    .checked_mul(std::mem::size_of::<f32>())
                    .and_then(|n| n.checked_add(filter_bytes))
                    .ok_or_else(write::limit_error)?
            }
        };
        add(&mut size, payload, limits)?;
        Ok(size)
    }
}

impl MemoryWriteRequest {
    pub(crate) fn storage_size(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        let mut size = store_size(&self.store, limits)?;
        match &self.operation {
            MemoryWriteOperation::Mutate { key, mutation } => {
                add(&mut size, key.as_str().len(), limits)?;
                let (MemoryMutation::Put { key, condition, .. }
                | MemoryMutation::Delete { key, condition }) = mutation;
                add(&mut size, limits.value_size(key)?, limits)?;
                add(&mut size, condition_size(condition), limits)?;
                if let MemoryMutation::Put { value, .. } = mutation {
                    add(&mut size, limits.value_size(value)?, limits)?;
                }
            }
            MemoryWriteOperation::Reconcile { operation } => {
                operation.validate()?;
                add(&mut size, operation.key.as_str().len(), limits)?;
                add(&mut size, operation.request_fingerprint.len(), limits)?;
            }
        }
        Ok(size)
    }
}

fn condition_size(condition: &WriteCondition) -> usize {
    condition
        .expected_version()
        .map_or(0, |v| v.as_token().len())
}
fn path_size(path: &[String], limits: &StorageLimits) -> Result<usize, HostError> {
    if path.len() > limits.max_nodes {
        return Err(write::limit_error());
    }
    let mut size = path
        .len()
        .checked_mul(std::mem::size_of::<String>())
        .ok_or_else(write::limit_error)?;
    for part in path {
        add(&mut size, part.len(), limits)?;
    }
    Ok(size)
}
fn store_size(store: &StoreRef, limits: &StorageLimits) -> Result<usize, HostError> {
    let mut size = path_size(&store.path, limits)?;
    add(&mut size, store.region.stable_id.len(), limits)?;
    if let Some(schema) = &store.region.schema_fingerprint {
        add(&mut size, schema.len(), limits)?;
    }
    Ok(size)
}
fn add(size: &mut usize, additional: usize, limits: &StorageLimits) -> Result<(), HostError> {
    *size = size
        .checked_add(additional)
        .filter(|n| *n <= limits.max_value_bytes)
        .ok_or_else(write::limit_error)?;
    Ok(())
}
