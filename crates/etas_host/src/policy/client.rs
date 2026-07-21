use std::future::Future;

use crate::{PolicyEvaluationRequest, PolicyResponse};

pub trait PolicyClient {
    type Error;
    type EvaluateFuture<'a>: Future<Output = Result<PolicyResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn evaluate(&self, request: PolicyEvaluationRequest) -> Self::EvaluateFuture<'_>;
}
