use std::{future::Future, pin::Pin};

use crate::{
    AuthorityContext, Budget, ByteStreamOrigin, HostError, HostErrorCode, HostRequestId,
    TcpStreamRef, TraceContext,
};

pub(crate) mod local;

pub use local::LocalTlsClient;

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

#[cfg(test)]
mod tests {
    use crate::{HostError, TlsClient};

    use super::LocalTlsClient;

    fn assert_tls_client<T: TlsClient<Error = HostError>>() {}

    #[test]
    fn local_tls_client_implements_service_trait() {
        assert_tls_client::<LocalTlsClient>();
    }
}
