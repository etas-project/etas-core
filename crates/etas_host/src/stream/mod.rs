use std::{fmt, future::Future, pin::Pin};

use crate::{
    AuthorityContext, ExecutionBudget, HostError, HostErrorCode, HostRequestId, TraceContext,
};

mod local;
pub(crate) mod store;

pub use local::LocalStreamClient;
pub use store::ByteStreamStore;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct StreamHandleRef {
    token: String,
    generation: u64,
}

impl StreamHandleRef {
    pub fn issued(token: impl Into<String>, generation: u64) -> Self {
        Self {
            token: token.into(),
            generation,
        }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn identity_fingerprint(&self) -> String {
        let mut input = self.token.as_bytes().to_vec();
        input.extend_from_slice(&self.generation.to_le_bytes());
        blake3::hash(&input).to_hex().to_string()
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }
}

impl fmt::Debug for StreamHandleRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StreamHandleRef")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByteStreamRef {
    handle: StreamHandleRef,
    origin: ByteStreamOrigin,
}

impl ByteStreamRef {
    pub fn issued(handle: StreamHandleRef, origin: ByteStreamOrigin) -> Self {
        Self { handle, origin }
    }

    pub fn opaque_for_testing(token: impl Into<String>, generation: u64) -> Self {
        Self::issued(
            StreamHandleRef::issued(token, generation),
            ByteStreamOrigin::Opaque,
        )
    }

    pub fn handle(&self) -> &StreamHandleRef {
        &self.handle
    }

    pub fn origin(&self) -> &ByteStreamOrigin {
        &self.origin
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
    pub budget: ExecutionBudget,
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
    pub result: Result<StreamPayload, StreamFailure>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamFailure {
    TimedOut,
    Cancelled,
    Closed,
    Interrupted,
    LimitExceeded { limit_bytes: usize },
    Host(HostError),
}

impl StreamFailure {
    pub(crate) fn from_host(error: HostError) -> Self {
        match error.code {
            HostErrorCode::TimedOut => Self::TimedOut,
            HostErrorCode::Cancelled => Self::Cancelled,
            HostErrorCode::Closed => Self::Closed,
            HostErrorCode::Interrupted => Self::Interrupted,
            _ => Self::Host(error),
        }
    }
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
                        result: Err(StreamFailure::LimitExceeded { limit_bytes: max }),
                    });
                }
                StreamOperation::ReadUntilLimit { limit_bytes, .. } if *limit_bytes > max => {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(StreamFailure::LimitExceeded { limit_bytes: max }),
                    });
                }
                _ => {}
            }
            Ok(StreamResponse {
                id: request.id,
                result: Err(StreamFailure::Host(HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "stream client is not configured",
                ))),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{HostError, StreamClient};

    use super::LocalStreamClient;

    fn assert_stream_client<T: StreamClient<Error = HostError>>() {}

    #[test]
    fn local_stream_client_implements_service_trait() {
        assert_stream_client::<LocalStreamClient>();
    }
}
