use super::receipt::StorageOperationRef;
use crate::HostError;

#[derive(Clone, Debug, PartialEq)]
pub enum WriteOutcome<R, N> {
    Committed(R),
    NotCommitted {
        operation: StorageOperationRef,
        reason: N,
    },
    Unknown {
        operation: StorageOperationRef,
        error: HostError,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConfirmedOutcome<R, N> {
    Committed(R),
    NotCommitted {
        operation: StorageOperationRef,
        reason: N,
    },
}

impl<R, N> TryFrom<WriteOutcome<R, N>> for ConfirmedOutcome<R, N> {
    type Error = HostError;

    fn try_from(outcome: WriteOutcome<R, N>) -> Result<Self, Self::Error> {
        match outcome {
            WriteOutcome::Committed(receipt) => Ok(Self::Committed(receipt)),
            WriteOutcome::NotCommitted { operation, reason } => {
                Ok(Self::NotCommitted { operation, reason })
            }
            WriteOutcome::Unknown { .. } => Err(HostError::new(
                crate::HostErrorCode::InvalidResponse,
                "unknown write outcome is not confirmed receipt evidence",
            )),
        }
    }
}

impl<R, N> From<ConfirmedOutcome<R, N>> for WriteOutcome<R, N> {
    fn from(outcome: ConfirmedOutcome<R, N>) -> Self {
        match outcome {
            ConfirmedOutcome::Committed(receipt) => Self::Committed(receipt),
            ConfirmedOutcome::NotCommitted { operation, reason } => {
                Self::NotCommitted { operation, reason }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageDurability {
    Volatile,
    SqliteWalFull,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReceiptLookup<R> {
    Found(R),
    Unresolved,
    Expired,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageWriteEvidence {
    pub operation: StorageOperationRef,
    pub status: CommitStatus,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommitStatus {
    Committed {
        revision: String,
        durability: StorageDurability,
    },
    NotCommitted,
    Unknown,
}
