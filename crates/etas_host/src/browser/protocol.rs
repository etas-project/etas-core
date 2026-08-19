use std::{future::Future, pin::Pin};

use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId, TraceContext,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserProtocolRequest {
    pub id: HostRequestId,
    pub operation: BrowserProtocolOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserProtocolOperation {
    Attach { profile: String },
    Create { profile: String },
    Send { session: String, message: Vec<u8> },
    Recv { session: String, max_bytes: usize },
    Screenshot { session: String, max_bytes: usize },
    Close { session: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserProtocolPayload {
    Session { id: String },
    Message(Vec<u8>),
    Screenshot(Vec<u8>),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserProtocolResponse {
    pub id: HostRequestId,
    pub result: Result<BrowserProtocolPayload, HostError>,
}

pub trait BrowserProtocolClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<BrowserProtocolResponse, Self::Error>>
        + Send
        + 'a
    where
        Self: 'a;

    fn execute(&self, request: BrowserProtocolRequest) -> Self::ExecuteFuture<'_>;
}

#[derive(Clone, Debug, Default)]
pub struct UnavailableBrowserProtocolClient;

impl BrowserProtocolClient for UnavailableBrowserProtocolClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<BrowserProtocolResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: BrowserProtocolRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            Ok(BrowserProtocolResponse {
                id: request.id,
                result: Err(HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "browser protocol client is not configured",
                )),
            })
        })
    }
}
