use super::SessionRetentionIntent;
use crate::*;
use std::future::Future;

/// Runtime control-plane service, deliberately separate from source Session APIs.
/// The embedding application authorizes maintenance before dispatch, as for other
/// Host service clients. Publication never invokes this service implicitly.
pub trait SessionMaintenanceClient {
    type MaintainFuture<'a>: Future<Output = Result<SessionMaintenanceResponse, HostError>>
        + Send
        + 'a
    where
        Self: 'a;
    fn maintain(&self, request: SessionMaintenanceRequest) -> Self::MaintainFuture<'_>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionMaintenanceRequest {
    pub id: HostRequestId,
    pub operation: SessionMaintenanceOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}
impl SessionMaintenanceRequest {
    pub(crate) fn validate(&self, limits: &StorageLimits) -> Result<usize, HostError> {
        self.authorize()?;
        match &self.operation {
            SessionMaintenanceOperation::Retain(intent) => intent.validate(limits),
            SessionMaintenanceOperation::Reconcile { session, operation } => {
                operation.validate()?;
                if session.id.is_empty() {
                    return Err(super::invalid("empty maintenance session identity"));
                }
                session
                    .id
                    .len()
                    .checked_add(operation.key.as_str().len())
                    .filter(|n| *n <= limits.max_value_bytes)
                    .ok_or_else(crate::session::write::limit_error)
            }
        }
    }
    pub(crate) fn authorize(&self) -> Result<(), HostError> {
        let (action, session) = match &self.operation {
            SessionMaintenanceOperation::Retain(intent) => ("write", &intent.session),
            SessionMaintenanceOperation::Reconcile { session, .. } => ("read", session),
        };
        if !self.authority.allows(&ActionInstance::new(
            "Memory",
            action,
            vec![HostValue::String(session.id.clone())],
        )) {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "session maintenance authority denied",
            ));
        }
        self.budget.check_time()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum SessionMaintenanceOperation {
    Retain(Box<SessionRetentionIntent>),
    Reconcile {
        session: SessionRef,
        operation: StorageOperationRef,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct SessionMaintenanceResponse {
    pub id: HostRequestId,
    pub result: Result<SessionMaintenanceResult, HostError>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SessionMaintenanceResult {
    Outcome(SessionRetentionOutcome),
    Receipt(ReceiptLookup<SessionRetentionReceipt>),
}
pub type SessionRetentionOutcome = WriteOutcome<SessionRetentionReceipt, SessionRetentionRejection>;
#[derive(Clone, Debug, PartialEq)]
pub enum SessionRetentionRejection {
    NoChange(SessionRetentionReceipt),
    Rejected(HostError),
}

/// Payload and dedup evidence are removed together. Commit receipts and published
/// context/provenance remain. A new append operation cannot claim deduplication
/// against deleted data; replay with the original operation key still uses its receipt.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionProgress {
    pub scanned: u32,
    pub deleted_messages: u32,
    pub deleted_dedup_keys: u32,
    pub next_after: Option<i64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SessionRetentionReceipt {
    pub operation: StorageOperationRef,
    pub session: SessionRef,
    pub generation: crate::session::SessionGeneration,
    pub progress: RetentionProgress,
    pub durability: StorageDurability,
}
impl SessionRetentionReceipt {
    pub(crate) fn check_result_size(&self, limits: &StorageLimits) -> Result<(), HostError> {
        if self.charge()? > limits.max_result_bytes {
            return Err(crate::session::write::limit_error());
        }
        Ok(())
    }
    pub fn into_outcome(self) -> SessionRetentionOutcome {
        if self.progress.deleted_messages == 0 {
            WriteOutcome::NotCommitted {
                operation: self.operation.clone(),
                reason: SessionRetentionRejection::NoChange(self),
            }
        } else {
            WriteOutcome::Committed(self)
        }
    }
    pub(crate) fn charge(&self) -> Result<usize, HostError> {
        crate::storage::receipt_budget::charge([
            self.session.id.as_str(),
            self.operation.key.as_str(),
            self.operation.request_fingerprint.as_str(),
            self.generation.as_token(),
        ])
    }
}

impl crate::execution::OperationResponse for SessionMaintenanceResponse {
    fn external_outcome(&self) -> crate::execution::ExternalOutcome {
        use crate::execution::ExternalOutcome;
        let Ok(SessionMaintenanceResult::Outcome(outcome)) = &self.result else {
            return if self.result.is_ok() {
                ExternalOutcome::Confirmed
            } else {
                ExternalOutcome::Unknown
            };
        };
        let (operation, status) = match outcome {
            WriteOutcome::Committed(r) => (
                &r.operation,
                CommitStatus::Committed {
                    revision: r.generation.as_token().to_owned(),
                    durability: r.durability,
                },
            ),
            WriteOutcome::NotCommitted { operation, .. } => (operation, CommitStatus::NotCommitted),
            WriteOutcome::Unknown { operation, .. } => (operation, CommitStatus::Unknown),
        };
        ExternalOutcome::StorageWrite(StorageWriteEvidence {
            operation: operation.clone(),
            status,
        })
    }
}
