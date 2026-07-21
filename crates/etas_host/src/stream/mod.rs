use std::{future::Future, pin::Pin};

use crate::{AuthorityContext, Budget, HostError, HostErrorCode, HostRequestId, TraceContext};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteStreamRef {
    pub id: String,
    pub origin: ByteStreamOrigin,
}

impl ByteStreamRef {
    pub fn new(id: impl Into<String>, origin: ByteStreamOrigin) -> Self {
        Self {
            id: id.into(),
            origin,
        }
    }

    pub fn opaque(id: impl Into<String>) -> Self {
        Self::new(id, ByteStreamOrigin::Opaque)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ByteStreamOrigin {
    Tcp {
        host: String,
        port: u16,
    },
    Tls {
        host: String,
        port: u16,
        server_name: Option<String>,
    },
    File {
        path: String,
    },
    Browser {
        session: String,
    },
    Opaque,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamRequest {
    pub id: HostRequestId,
    pub operation: StreamOperation,
    pub authority: AuthorityContext,
    pub trace: TraceContext,
    pub budget: Budget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamOperation {
    Read {
        stream: ByteStreamRef,
        max_bytes: usize,
        timeout_ms: Option<u64>,
    },
    ReadUntilLimit {
        stream: ByteStreamRef,
        limit_bytes: usize,
        timeout_ms: Option<u64>,
    },
    WriteAll {
        stream: ByteStreamRef,
        body: Vec<u8>,
    },
    Flush {
        stream: ByteStreamRef,
    },
    Close {
        stream: ByteStreamRef,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamResponse {
    pub id: HostRequestId,
    pub result: Result<StreamPayload, HostError>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamPayload {
    Read(StreamRead),
    Unit,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamRead {
    Data(Vec<u8>),
    Eof,
}

pub trait StreamClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<StreamResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: StreamRequest) -> Self::ExecuteFuture<'_>;
}

#[derive(Clone, Debug)]
pub struct UnavailableStreamClient {
    max_read_bytes: usize,
}

impl UnavailableStreamClient {
    pub fn new(max_read_bytes: usize) -> Self {
        Self { max_read_bytes }
    }
}

impl Default for UnavailableStreamClient {
    fn default() -> Self {
        Self {
            max_read_bytes: 1024 * 1024,
        }
    }
}

impl StreamClient for UnavailableStreamClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<StreamResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: StreamRequest) -> Self::ExecuteFuture<'_> {
        let max = self.max_read_bytes;
        Box::pin(async move {
            match &request.operation {
                StreamOperation::Read { max_bytes, .. } if *max_bytes > max => {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::BudgetExceeded,
                            "stream read exceeds configured maximum",
                        )),
                    });
                }
                StreamOperation::ReadUntilLimit { limit_bytes, .. } if *limit_bytes > max => {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::BudgetExceeded,
                            "stream read-until-limit exceeds configured maximum",
                        )),
                    });
                }
                _ => {}
            }
            Ok(StreamResponse {
                id: request.id,
                result: Err(HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "stream client is not configured",
                )),
            })
        })
    }
}
