use std::{future::Future, pin::Pin};

use crate::{
    AuthorityContext, ByteStreamOrigin, ExecutionBudget, HostError, HostErrorCode, HostRequestId,
    StreamHandleRef, TcpStreamRef, TraceContext,
};

pub(crate) mod local;

pub use local::LocalTlsClient;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsStreamRef {
    stream: crate::ByteStreamRef,
}

impl TlsStreamRef {
    pub fn issued(handle: StreamHandleRef, origin: ByteStreamOrigin) -> Self {
        Self {
            stream: crate::ByteStreamRef::issued(handle, origin),
        }
    }

    pub fn handle(&self) -> &StreamHandleRef {
        self.stream.handle()
    }

    pub fn origin(&self) -> &ByteStreamOrigin {
        self.stream.origin()
    }

    pub fn as_byte_stream(&self) -> crate::ByteStreamRef {
        self.stream.clone()
    }

    pub fn with_read_scope(&self, scope: &crate::StreamReadScope) -> Self {
        Self {
            stream: self.stream.clone().with_read_scope(scope),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TlsConnectRequest {
    pub id: HostRequestId,
    pub operation: TlsConnectOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TlsConnectOperation {
    Connect {
        stream: TcpStreamRef,
        server_name: String,
        options: TlsConnectOptions,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TlsVersion {
    #[default]
    V12,
    V13,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TlsTrust {
    #[default]
    Platform,
    Bundle(Vec<u8>),
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TlsConnectOptions {
    pub timeout_ms: Option<u128>,
    pub minimum: TlsVersion,
    pub trust: TlsTrust,
    pub alpn: Vec<String>,
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
