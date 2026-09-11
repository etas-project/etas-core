use super::*;
use crate::StorageLimits;
use crate::memory::{
    MemoryMutation, MemoryNotCommitted, MemoryWriteChange, MemoryWriteOperation,
    MemoryWriteReceipt, MemoryWriteRequest, MemoryWriteResponse, MemoryWriteResult,
};
use crate::storage::{
    outcome::{ConfirmedOutcome, ReceiptLookup, StorageDurability, WriteOutcome},
    receipt::StorageOperationRef,
};

impl InMemoryMemoryClient {
    pub async fn write_scoped(
        &self,
        request: MemoryWriteRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<MemoryWriteResponse, HostError> {
        self.dispatch_write(request, Some(operation)).await
    }

    pub(super) async fn dispatch_write(
        &self,
        request: MemoryWriteRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<MemoryWriteResponse, HostError> {
        let bytes = request.storage_size(&self.limits)?;
        let id = request.id;
        let pending = match &request.operation {
            MemoryWriteOperation::Mutate { key, mutation } => {
                Some(mutation.operation_ref(&request.store, key.clone(), &self.limits)?)
            }
            MemoryWriteOperation::Reconcile { operation } => {
                operation.validate()?;
                None
            }
        };
        let client = self.clone();
        let result = self
            .executor
            .execute(
                operation,
                request.id,
                request.trace.clone(),
                request.budget.clone(),
                bytes,
                move |_| {
                    Ok(MemoryWriteResponse {
                        id: request.id,
                        result: client.execute_write(request),
                    })
                },
            )
            .await;
        match (result, pending) {
            (Err(error), Some(operation)) => Ok(MemoryWriteResponse {
                id,
                result: Ok(MemoryWriteResult::Outcome(
                    WriteOutcome::from_dispatch_error(
                        error,
                        operation,
                        MemoryNotCommitted::Rejected,
                    ),
                )),
            }),
            (result, _) => result.map_err(crate::execution::DispatchError::into_host_error),
        }
    }

    pub(super) fn execute_write(
        &self,
        request: MemoryWriteRequest,
    ) -> Result<MemoryWriteResult, HostError> {
        let mut receipts = self.receipts.lock().map_err(lock_error)?;
        let now = crate::storage::receipt::now()?;
        receipts.retain(|_, outcome| operation_ref(outcome).key.expires_at() > now);
        let scope = store_key(&request.store);
        match request.operation {
            MemoryWriteOperation::Reconcile { operation } => {
                operation.validate()?;
                let result = if operation.key.is_expired()? {
                    ReceiptLookup::Expired
                } else if let Some(outcome) =
                    receipts.get(&(scope, operation.key.as_str().to_owned()))
                {
                    if operation_ref(outcome) != &operation {
                        return Err(crate::memory::write::mismatch());
                    }
                    if let ConfirmedOutcome::Committed(receipt) = outcome
                        && receipt.target.store != request.store
                    {
                        return Err(crate::memory::write::mismatch());
                    }
                    ReceiptLookup::Found(outcome.clone())
                } else {
                    ReceiptLookup::Unresolved
                };
                Ok(MemoryWriteResult::Receipt(result))
            }
            MemoryWriteOperation::Mutate { key, mutation } => {
                let target = mutation.target(&request.store);
                let operation = mutation.operation_ref(&request.store, key, &self.limits)?;
                operation
                    .key
                    .validate_window(self.limits.max_receipt_retention_seconds)?;
                let identity = (scope, operation.key.as_str().to_owned());
                if let Some(existing) = receipts.get(&identity) {
                    if operation_ref(existing) != &operation {
                        return Err(crate::memory::write::mismatch());
                    }
                    return Ok(MemoryWriteResult::Outcome(existing.clone().into()));
                }
                if receipts.len() >= self.limits.max_receipts {
                    return Ok(MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
                        operation,
                        reason: MemoryNotCommitted::Rejected(crate::memory::write::limit_error()),
                    }));
                }
                let admission = (|| {
                    let used = crate::storage::receipt_budget::sum(receipts.iter().map(
                        |((scope, _), outcome)| {
                            receipt_charge(
                                scope,
                                operation_ref(outcome),
                                match outcome {
                                    ConfirmedOutcome::Committed(receipt) => Some(&receipt.target),
                                    _ => None,
                                },
                                &self.limits,
                            )
                        },
                    ))?;
                    crate::storage::receipt_budget::admit(
                        &self.limits,
                        used,
                        receipt_charge(&identity.0, &operation, Some(&target), &self.limits)?,
                    )
                })();
                if let Err(error) = admission {
                    return Ok(MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
                        operation,
                        reason: MemoryNotCommitted::Rejected(error),
                    }));
                }
                let result = match mutation {
                    MemoryMutation::Put {
                        key,
                        value,
                        condition,
                    } => self.put(&request.store, key, value, condition),
                    MemoryMutation::Delete { key, condition } => {
                        self.delete(&request.store, key, condition)
                    }
                };
                let outcome = match result {
                    Ok(MemoryResult::Written { version }) => {
                        ConfirmedOutcome::Committed(MemoryWriteReceipt {
                            operation,
                            target,
                            change: MemoryWriteChange::Written { version },
                            durability: StorageDurability::Volatile,
                        })
                    }
                    Ok(MemoryResult::Deleted { version }) => {
                        ConfirmedOutcome::Committed(MemoryWriteReceipt {
                            operation,
                            target,
                            change: MemoryWriteChange::Deleted { tombstone: version },
                            durability: StorageDurability::Volatile,
                        })
                    }
                    Ok(MemoryResult::Conflict(conflict)) => ConfirmedOutcome::NotCommitted {
                        operation,
                        reason: MemoryNotCommitted::Conflict {
                            expected: conflict.expected,
                            actual: conflict.actual,
                            current_value: conflict.current_value,
                        },
                    },
                    Ok(MemoryResult::Unchanged) => ConfirmedOutcome::NotCommitted {
                        operation,
                        reason: MemoryNotCommitted::Unchanged,
                    },
                    Err(error) => {
                        return Ok(MemoryWriteResult::Outcome(WriteOutcome::NotCommitted {
                            operation,
                            reason: MemoryNotCommitted::Rejected(error),
                        }));
                    }
                    Ok(_) => {
                        return Err(HostError::new(
                            HostErrorCode::SchemaMismatch,
                            "mutation produced a read result",
                        ));
                    }
                };
                receipts.insert(identity, outcome.clone());
                Ok(MemoryWriteResult::Outcome(outcome.into()))
            }
        }
    }
}

fn operation_ref(outcome: &crate::memory::MemoryConfirmedOutcome) -> &StorageOperationRef {
    match outcome {
        ConfirmedOutcome::Committed(receipt) => &receipt.operation,
        ConfirmedOutcome::NotCommitted { operation, .. } => operation,
    }
}

fn receipt_charge(
    scope: &StoreKey,
    operation: &StorageOperationRef,
    target: Option<&crate::memory::MemoryWriteTarget>,
    limits: &StorageLimits,
) -> Result<usize, HostError> {
    let path = serde_json::to_string(&scope.path).map_err(|error| {
        HostError::new(HostErrorCode::InvalidRequest, "invalid receipt Store path")
            .with_detail("error", error.to_string())
    })?;
    let key = target
        .map(|target| crate::value::tagged::encode_with_limits(&target.key, limits))
        .transpose()?;
    crate::storage::receipt_budget::charge([
        scope.region.as_str(),
        path.as_str(),
        operation.key.as_str(),
        operation.request_fingerprint.as_str(),
        key.as_deref().unwrap_or(""),
        target
            .and_then(|target| target.store.region.schema_fingerprint.as_deref())
            .unwrap_or(""),
    ])
}
