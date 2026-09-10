mod codec;

use super::{MemoryMutation, MemoryWriteOperation, MemoryWriteRequest};
use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId, HostValue,
    StorageLimits, StorageOperationKey, StorageOperationRef, StoreRef, TraceContext,
    WriteCondition,
};

/// Immutable data and pre-dispatch identity, without authority or live resources.
#[derive(Clone, PartialEq)]
pub struct MemoryWriteIntent {
    store: StoreRef,
    mutation: MemoryMutation,
    operation: StorageOperationRef,
}

impl std::fmt::Debug for MemoryWriteIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryWriteIntent").finish_non_exhaustive()
    }
}

impl MemoryWriteIntent {
    pub fn prepare_put(
        store: StoreRef,
        key: HostValue,
        value: HostValue,
        condition: WriteCondition,
        limits: &StorageLimits,
    ) -> Result<Self, HostError> {
        Self::prepare(
            store,
            MemoryMutation::Put {
                key,
                value,
                condition,
            },
            limits,
        )
    }

    pub fn prepare_delete(
        store: StoreRef,
        key: HostValue,
        condition: WriteCondition,
        limits: &StorageLimits,
    ) -> Result<Self, HostError> {
        Self::prepare(store, MemoryMutation::Delete { key, condition }, limits)
    }

    fn prepare(
        store: StoreRef,
        mutation: MemoryMutation,
        limits: &StorageLimits,
    ) -> Result<Self, HostError> {
        validate_target(&store)?;
        limits.validate()?;
        let key = StorageOperationKey::new(std::time::Duration::from_secs(
            limits.max_receipt_retention_seconds,
        ))?;
        let operation = mutation.operation_ref(&store, key, limits)?;
        let intent = Self {
            store,
            mutation,
            operation,
        };
        // Preparation promises a bounded transportable intent, not just a
        // mutation whose payload fits when the envelope is ignored.
        intent.encode(limits)?;
        Ok(intent)
    }

    pub fn operation_ref(&self) -> &StorageOperationRef {
        &self.operation
    }

    pub fn store(&self) -> &StoreRef {
        &self.store
    }

    pub fn mutation(&self) -> &MemoryMutation {
        &self.mutation
    }

    pub fn encode(&self, limits: &StorageLimits) -> Result<String, HostError> {
        self.validate(limits)?;
        codec::encode(self, limits)
    }

    pub fn decode(encoded: &str, limits: &StorageLimits) -> Result<Self, HostError> {
        limits.validate()?;
        let intent = codec::decode(encoded, limits)?;
        intent.validate(limits)?;
        Ok(intent)
    }

    /// The caller supplies the current invocation's authority, trace and budget.
    /// Admission and replay still go through the normal MemoryClient write path.
    pub fn into_request(
        self,
        id: HostRequestId,
        authority: AuthorityContext,
        trace: TraceContext,
        budget: ExecutionBudget,
        limits: &StorageLimits,
    ) -> Result<MemoryWriteRequest, HostError> {
        self.encode(limits)?;
        self.operation
            .key
            .validate_window(limits.max_receipt_retention_seconds)?;
        Ok(MemoryWriteRequest {
            id,
            store: self.store,
            operation: MemoryWriteOperation::Mutate {
                key: self.operation.key,
                mutation: self.mutation,
            },
            authority,
            trace,
            budget,
        })
    }

    pub fn validate(&self, limits: &StorageLimits) -> Result<(), HostError> {
        limits.validate()?;
        validate_target(&self.store)?;
        self.operation.validate()?;
        let expected =
            self.mutation
                .operation_ref(&self.store, self.operation.key.clone(), limits)?;
        if expected != self.operation {
            return Err(super::write::mismatch());
        }
        Ok(())
    }
}

fn validate_target(store: &StoreRef) -> Result<(), HostError> {
    if store.region.stable_id.is_empty()
        || store.path.iter().any(String::is_empty)
        || store
            .region
            .schema_fingerprint
            .as_ref()
            .is_some_and(String::is_empty)
    {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "invalid memory write intent")
}
