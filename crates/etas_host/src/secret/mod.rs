use std::{future::Future, pin::Pin};

use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId, TraceContext,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SecretRequest {
    pub id: HostRequestId,
    pub operation: SecretOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretOperation {
    Read { key: String },
    HmacSha256 { key: SecretRef, body: Vec<u8> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretValue {
    reference: SecretRef,
    redacted: String,
}

impl SecretValue {
    pub fn new(reference: SecretRef, redacted: impl Into<String>) -> Self {
        Self {
            reference,
            redacted: redacted.into(),
        }
    }

    pub fn reference(&self) -> &SecretRef {
        &self.reference
    }

    pub fn redacted_label(&self) -> &str {
        &self.redacted
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SecretRef {
    id: String,
}

impl SecretRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecretResponse {
    pub id: HostRequestId,
    pub result: Result<SecretPayload, HostError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecretPayload {
    Value(SecretValue),
    Bytes(Vec<u8>),
}

pub trait SecretClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<SecretResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: SecretRequest) -> Self::ExecuteFuture<'_>;
}

#[derive(Clone, Debug, Default)]
pub struct UnavailableSecretClient;

impl SecretClient for UnavailableSecretClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<SecretResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: SecretRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            Ok(SecretResponse {
                id: request.id,
                result: Err(HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "secret client is not configured",
                )),
            })
        })
    }
}
