use crate::storage::{
    outcome::{ConfirmedOutcome, ReceiptLookup, StorageDurability, WriteOutcome},
    receipt::{StorageOperationKey, StorageOperationRef},
};
use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId, HostValue,
    MemoryVersion, StorageLimits, StoreRef, TraceContext, WriteCondition,
};

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryMutation {
    Put {
        key: HostValue,
        value: HostValue,
        condition: WriteCondition,
    },
    Delete {
        key: HostValue,
        condition: WriteCondition,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryWriteRequest {
    pub id: HostRequestId,
    pub store: StoreRef,
    pub operation: MemoryWriteOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryWriteOperation {
    Mutate {
        key: StorageOperationKey,
        mutation: MemoryMutation,
    },
    Reconcile {
        operation: StorageOperationRef,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryWriteResponse {
    pub id: HostRequestId,
    pub result: Result<MemoryWriteResult, HostError>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryWriteResult {
    Outcome(MemoryWriteOutcome),
    Receipt(ReceiptLookup<MemoryConfirmedOutcome>),
}
pub type MemoryWriteOutcome = WriteOutcome<MemoryWriteReceipt, MemoryNotCommitted>;
pub type MemoryConfirmedOutcome = ConfirmedOutcome<MemoryWriteReceipt, MemoryNotCommitted>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryMutationKind {
    Put,
    Delete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryWriteReceipt {
    pub operation: StorageOperationRef,
    pub target: MemoryWriteTarget,
    pub change: MemoryWriteChange,
    pub durability: StorageDurability,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryWriteTarget {
    pub store: StoreRef,
    pub key: HostValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MemoryWriteChange {
    Written { version: MemoryVersion },
    Deleted { tombstone: MemoryVersion },
}

impl MemoryWriteChange {
    pub fn kind(&self) -> MemoryMutationKind {
        match self {
            Self::Written { .. } => MemoryMutationKind::Put,
            Self::Deleted { .. } => MemoryMutationKind::Delete,
        }
    }

    pub fn revision(&self) -> &MemoryVersion {
        match self {
            Self::Written { version } => version,
            Self::Deleted { tombstone } => tombstone,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MemoryNotCommitted {
    Conflict {
        expected: Option<MemoryVersion>,
        actual: Option<MemoryVersion>,
        current_value: Option<HostValue>,
    },
    Unchanged,
    Rejected(HostError),
}

impl MemoryMutation {
    pub fn target(&self, store: &StoreRef) -> MemoryWriteTarget {
        let (Self::Put { key, .. } | Self::Delete { key, .. }) = self;
        MemoryWriteTarget {
            store: store.clone(),
            key: key.clone(),
        }
    }

    pub fn operation_ref(
        &self,
        store: &StoreRef,
        key: StorageOperationKey,
        limits: &StorageLimits,
    ) -> Result<StorageOperationRef, HostError> {
        let (kind, item, value, condition) = match self {
            Self::Put {
                key,
                value,
                condition,
            } => ("put", key, Some(value), condition),
            Self::Delete { key, condition } => ("delete", key, None, condition),
        };
        let condition = match condition {
            WriteCondition::Any => "any",
            WriteCondition::Missing => "missing",
            WriteCondition::Exists => "exists",
            WriteCondition::Match(version) => version.as_token(),
        };
        let mut hash = blake3::Hasher::new();
        hash.update(b"etas.memory.mutation.v1");
        hash.update(&[u8::from(store.region.schema_fingerprint.is_some())]);
        hash.update(&(store.path.len() as u64).to_le_bytes());
        let mut total = 0usize;
        for part in std::iter::once(store.region.stable_id.as_str())
            .chain(store.region.schema_fingerprint.as_deref())
            .chain(store.path.iter().map(String::as_str))
            .chain([kind])
        {
            hash_part_length(&mut hash, part.len(), &mut total, limits)?;
            hash.update(part.as_bytes());
        }
        hash_value(&mut hash, item, &mut total, limits)?;
        hash_part_length(&mut hash, condition.len(), &mut total, limits)?;
        hash.update(condition.as_bytes());
        if let Some(value) = value {
            hash_value(&mut hash, value, &mut total, limits)?;
        }
        Ok(StorageOperationRef {
            key,
            request_fingerprint: hash.finalize().to_hex().to_string(),
        })
    }
}
fn hash_value(
    hash: &mut blake3::Hasher,
    value: &HostValue,
    total: &mut usize,
    limits: &StorageLimits,
) -> Result<(), HostError> {
    // v1 fingerprints prefix each canonical encoding with its length. Count
    // without retaining bytes, then stream that same encoding into the hash.
    let length = crate::value::tagged::encode_to(value, limits, std::io::sink())?;
    hash_part_length(hash, length, total, limits)?;
    crate::value::tagged::encode_to(value, limits, hash)?;
    Ok(())
}
fn hash_part_length(
    hash: &mut blake3::Hasher,
    length: usize,
    total: &mut usize,
    limits: &StorageLimits,
) -> Result<(), HostError> {
    *total = total.checked_add(length).ok_or_else(limit_error)?;
    if *total > limits.max_value_bytes {
        return Err(limit_error());
    }
    hash.update(&(length as u64).to_le_bytes());
    Ok(())
}
pub(crate) fn limit_error() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "memory mutation or receipt storage limit exceeded",
    )
}
pub(crate) fn mismatch() -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "storage operation identity is bound to a different request",
    )
}
