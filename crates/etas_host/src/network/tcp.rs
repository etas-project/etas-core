use std::{future::Future, pin::Pin};

use crate::{
    AuthorityContext, Budget, ByteStreamOrigin, HostError, HostErrorCode, HostRequestId,
    SandboxBroker, TraceContext,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpStreamRef {
    pub id: String,
    pub origin: ByteStreamOrigin,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TcpConnectRequest {
    pub id: HostRequestId,
    pub operation: TcpConnectOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TcpConnectOperation {
    Connect { endpoint: TcpEndpoint },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpConnectResponse {
    pub id: HostRequestId,
    pub result: Result<TcpStreamRef, HostError>,
}

pub trait TcpClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<TcpConnectResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: TcpConnectRequest) -> Self::ExecuteFuture<'_>;
}

#[derive(Clone, Debug, Default)]
pub struct UnavailableTcpClient;

impl TcpClient for UnavailableTcpClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<TcpConnectResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: TcpConnectRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(async move {
            let policy_check = match &request.operation {
                TcpConnectOperation::Connect { endpoint } => {
                    SandboxBroker::new(request.authority.sandbox.clone())
                        .resolve_network_endpoint("tcp", &endpoint.host, endpoint.port)
                        .map(|_| ())
                }
            };
            if let Err(error) = policy_check {
                return Ok(TcpConnectResponse {
                    id: request.id,
                    result: Err(error),
                });
            }
            Ok(TcpConnectResponse {
                id: request.id,
                result: Err(HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "TCP client is not configured",
                )),
            })
        })
    }
}
