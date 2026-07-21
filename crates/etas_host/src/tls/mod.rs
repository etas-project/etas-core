use std::{future::Future, pin::Pin};

use crate::{
    AuthorityContext, Budget, ByteStreamOrigin, HostError, HostErrorCode, HostRequestId,
    TcpStreamRef, TraceContext,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsStreamRef {
    pub id: String,
    pub origin: ByteStreamOrigin,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TlsConnectRequest {
    pub id: HostRequestId,
    pub operation: TlsConnectOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TlsConnectOperation {
    Connect {
        stream: TcpStreamRef,
        server_name: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsConnectResponse {
    pub id: HostRequestId,
    pub result: Result<TlsStreamRef, HostError>,
}

pub trait TlsClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<TlsConnectResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: TlsConnectRequest) -> Self::ExecuteFuture<'_>;
}

#[derive(Clone, Debug, Default)]
pub struct UnavailableTlsClient;

impl TlsClient for UnavailableTlsClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<TlsConnectResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: TlsConnectRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            Ok(TlsConnectResponse {
                id: request.id,
                result: Err(HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "TLS client is not configured",
                )),
            })
        })
    }
}
