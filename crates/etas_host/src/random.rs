//! Cryptographic operating-system entropy; no deterministic fallback.
use crate::{
    ActionInstance, AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId,
    HostTraceFieldSensitivity, HostTracePayload, HostTraceRequest, HostValue, TraceContext,
};

#[derive(Clone, Debug, PartialEq)]
pub struct EntropyRequest {
    pub id: HostRequestId,
    pub count: usize,
    pub max_bytes: usize,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, PartialEq, Eq)]
pub struct EntropyResponse {
    pub id: HostRequestId,
    pub result: Result<Vec<u8>, HostError>,
}

impl std::fmt::Debug for EntropyResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntropyResponse")
            .field("id", &self.id)
            .field("result", &self.result.as_ref().map(Vec::len))
            .finish()
    }
}

impl HostTraceRequest for EntropyRequest {
    fn trace_payload(&self) -> HostTracePayload {
        HostTracePayload::new("random", "Random.entropy").with_field(
            "count",
            HostValue::UInt(self.count as u128),
            HostTraceFieldSensitivity::Public,
        )
    }
}

pub async fn entropy(
    request: EntropyRequest,
    operation: &crate::execution::OperationContext,
) -> Result<EntropyResponse, HostError> {
    request.budget.check_time()?;
    operation.signal().check()?;
    if !request
        .authority
        .allows(&ActionInstance::new("Random", "entropy", Vec::new()))
    {
        return Err(HostError::new(
            HostErrorCode::AuthorityDenied,
            "entropy request requires the current Random.entropy action grant",
        ));
    }
    if request.count > request.max_bytes {
        return Err(HostError::new(
            HostErrorCode::BudgetExceeded,
            "entropy result exceeds the run byte limit",
        ));
    }
    operation
        .run_blocking(request.budget.clone(), move |operation| {
            let mut bytes = vec![0; request.count];
            let result = getrandom::fill(&mut bytes).map(|()| bytes).map_err(|_| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "operating-system entropy is unavailable",
                )
            });
            operation.signal().check()?;
            request.budget.check_time()?;
            Ok(EntropyResponse {
                id: request.id,
                result,
            })
        })
        .await
}
