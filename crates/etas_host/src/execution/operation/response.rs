use super::ExternalOutcome;

/// A response is evidence, not proof that a failed operation had no side effects.
pub trait OperationResponse {
    fn external_outcome(&self) -> ExternalOutcome;
}

impl OperationResponse for crate::session::SessionWriteResponse {
    fn external_outcome(&self) -> ExternalOutcome {
        use crate::session::SessionWriteResult;
        use crate::{CommitStatus, StorageWriteEvidence, WriteOutcome};
        if let Ok(SessionWriteResult::Context(outcome)) = &self.result {
            let (operation, status) = match outcome {
                WriteOutcome::Committed(receipt) => (
                    &receipt.operation,
                    CommitStatus::Committed {
                        revision: receipt.generation.as_token().to_owned(),
                        durability: receipt.durability,
                    },
                ),
                WriteOutcome::NotCommitted { operation, .. } => {
                    (operation, CommitStatus::NotCommitted)
                }
                WriteOutcome::Unknown { operation, .. } => (operation, CommitStatus::Unknown),
            };
            return ExternalOutcome::StorageWrite(StorageWriteEvidence {
                operation: operation.clone(),
                status,
            });
        }
        let Ok(SessionWriteResult::Outcome(outcome)) = &self.result else {
            return if self.result.is_ok() {
                ExternalOutcome::Confirmed
            } else {
                ExternalOutcome::Unknown
            };
        };
        let (operation, status) = match outcome {
            WriteOutcome::Committed(receipt) => (
                receipt.operation(),
                CommitStatus::Committed {
                    revision: receipt.revision(),
                    durability: receipt.durability(),
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

impl OperationResponse for crate::memory::MemoryWriteResponse {
    fn external_outcome(&self) -> ExternalOutcome {
        use crate::memory::MemoryWriteResult;
        use crate::storage::outcome::{CommitStatus, StorageWriteEvidence, WriteOutcome};
        let Ok(MemoryWriteResult::Outcome(outcome)) = &self.result else {
            return if self.result.is_ok() {
                ExternalOutcome::Confirmed
            } else {
                ExternalOutcome::Unknown
            };
        };
        let (operation, status) = match outcome {
            WriteOutcome::Committed(receipt) => (
                &receipt.operation,
                CommitStatus::Committed {
                    revision: receipt.change.revision().as_token().to_owned(),
                    durability: receipt.durability,
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

macro_rules! result_response {
    ($($ty:ty),* $(,)?) => { $(impl OperationResponse for $ty {
        fn external_outcome(&self) -> ExternalOutcome {
            if self.result.is_ok() { ExternalOutcome::Confirmed } else { ExternalOutcome::Unknown }
        }
    })* };
}
result_response!(
    crate::CommandResponse,
    crate::FilesystemResponse,
    crate::MemoryResponse,
    crate::SessionResponse,
    crate::StreamResponse,
    crate::TcpConnectResponse,
    crate::TlsConnectResponse,
    crate::ToolResponse,
    crate::SecretResponse,
    crate::BrowserProtocolResponse
);

macro_rules! confirmed_response {
    ($($ty:ty),* $(,)?) => { $(impl OperationResponse for $ty {
        fn external_outcome(&self) -> ExternalOutcome { ExternalOutcome::Confirmed }
    })* };
}
confirmed_response!(
    crate::ModelResponse,
    crate::console::ConsoleResponse,
    crate::ApprovalResponse,
    crate::PolicyResponse
);
