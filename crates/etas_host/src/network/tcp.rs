use std::{future::Future, pin::Pin};

use crate::{
    AuthorityContext, ByteStreamOrigin, ExecutionBudget, HostError, HostErrorCode, HostRequestId,
    SandboxBroker, StreamHandleRef, TraceContext,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpStreamRef {
    stream: crate::ByteStreamRef,
}

impl TcpStreamRef {
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
pub struct TcpConnectRequest {
    pub id: HostRequestId,
    pub operation: TcpConnectOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: ExecutionBudget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TcpConnectOperation {
    Connect {
        endpoint: TcpEndpoint,
        options: TcpConnectOptions,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TcpConnectOptions {
    pub timeout_ms: Option<u128>,
    pub nodelay: bool,
}

impl Default for TcpConnectOptions {
    fn default() -> Self {
        Self {
            timeout_ms: None,
            nodelay: true,
        }
    }
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
                TcpConnectOperation::Connect { endpoint, .. } => {
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
