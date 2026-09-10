use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId, ReceiptLookup,
    SessionMessage, SessionRef, StorageDurability, StorageLimits, StorageOperationKey,
    StorageOperationRef, TraceContext, WriteOutcome,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SessionWriteRequest {
    pub id: HostRequestId,
    pub operation: SessionWriteOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionWriteOperation {
    PublishContext(Box<super::SessionContextPublication>),
    ReconcileContext {
        session: SessionRef,
        operation: StorageOperationRef,
    },
    Resolve {
        key: StorageOperationKey,
        config: crate::SessionConfig,
    },
    Append {
        key: StorageOperationKey,
        message: Box<SessionMessage>,
    },
    Reconcile {
        session: SessionRef,
        operation: StorageOperationRef,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionWriteResponse {
    pub id: HostRequestId,
    pub result: Result<SessionWriteResult, HostError>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionWriteResult {
    Context(super::SessionContextOutcome),
    ContextReceipt(ReceiptLookup<super::SessionContextEvidence>),
    Outcome(SessionWriteOutcome),
    Receipt(ReceiptLookup<SessionWriteReceipt>),
}
pub type SessionWriteOutcome = WriteOutcome<SessionWriteReceipt, HostError>;

#[derive(Clone, Debug, PartialEq)]
pub enum SessionWriteReceipt {
    Append(SessionAppendReceipt),
    Resolve(SessionResolveReceipt),
}
impl SessionWriteReceipt {
    pub fn operation(&self) -> &StorageOperationRef {
        match self {
            Self::Append(r) => &r.operation,
            Self::Resolve(r) => &r.operation,
        }
    }
    pub fn durability(&self) -> StorageDurability {
        match self {
            Self::Append(r) => r.durability,
            Self::Resolve(r) => r.durability,
        }
    }
    pub fn revision(&self) -> String {
        match self {
            Self::Append(r) => r.version.as_token().to_owned(),
            Self::Resolve(r) => r.generation.as_token().to_owned(),
        }
    }
    pub(crate) fn charge(&self, session: &str) -> Result<usize, HostError> {
        let identity = match self {
            Self::Append(r) => r.message_id.as_str(),
            Self::Resolve(r) => r.session.id.as_str(),
        };
        let operation = self.operation();
        let base = crate::storage::receipt_budget::charge([
            session,
            operation.key.as_str(),
            operation.request_fingerprint.as_str(),
            identity,
        ])?;
        Ok(base)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionResolveReceipt {
    pub operation: StorageOperationRef,
    pub session: SessionRef,
    pub created: bool,
    pub generation: super::SessionGeneration,
    pub durability: StorageDurability,
}

pub fn resolve_operation_ref(
    config: &crate::SessionConfig,
    key: StorageOperationKey,
    limits: &StorageLimits,
) -> Result<StorageOperationRef, HostError> {
    if config.id.is_empty() {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "session id must not be empty",
        ));
    }
    if config.id.len() > limits.max_value_bytes {
        return Err(limit_error());
    }
    let mut hash = blake3::Hasher::new_derive_key("etas.session.resolve.v2");
    hash.update(&(config.id.len() as u64).to_le_bytes());
    hash.update(config.id.as_bytes());
    let (tag, count) = match config.context {
        crate::ContextPolicy::All => (0, 0),
        crate::ContextPolicy::LastTurns(n) => (1, u64::try_from(n).map_err(|_| limit_error())?),
        crate::ContextPolicy::SummaryPlusRecent { recent } => {
            (2, u64::try_from(recent).map_err(|_| limit_error())?)
        }
    };
    hash.update(&[tag]);
    hash.update(&count.to_le_bytes());
    let (tag, count) = match config.retention {
        crate::RetentionPolicy::Forever => (0, 0),
        crate::RetentionPolicy::Days(n) => (1, n),
    };
    hash.update(&[tag]);
    hash.update(&count.to_le_bytes());
    Ok(StorageOperationRef {
        key,
        request_fingerprint: hash.finalize().to_hex().to_string(),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionAppendReceipt {
    pub operation: StorageOperationRef,
    pub version: super::SessionVersion,
    pub message_id: String,
    pub deduplicated: bool,
    pub durability: StorageDurability,
}

pub fn append_operation_ref(
    message: &SessionMessage,
    key: StorageOperationKey,
    limits: &StorageLimits,
) -> Result<StorageOperationRef, HostError> {
    let size = message.storage_size(limits)?;
    if size > limits.max_value_bytes {
        return Err(limit_error());
    }
    let mut hash = blake3::Hasher::new_derive_key("etas.session.append.v1");
    crate::value::tagged::encode_record_to(
        &super::value::session_message_fields(message),
        limits,
        &mut hash,
    )?;
    Ok(StorageOperationRef {
        key,
        request_fingerprint: hash.finalize().to_hex().to_string(),
    })
}
pub(crate) fn limit_error() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "session mutation or receipt storage limit exceeded",
    )
}
pub(crate) fn mismatch() -> HostError {
    HostError::new(
        HostErrorCode::InvalidRequest,
        "session operation identity is bound to a different request",
    )
}
