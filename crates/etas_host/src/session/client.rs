use std::future::Future;

use crate::{SessionRequest, SessionResponse};

pub trait SessionClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<SessionResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: SessionRequest) -> Self::ExecuteFuture<'_>;

    type WriteFuture<'a>: Future<Output = Result<super::SessionWriteResponse, Self::Error>>
        + Send
        + 'a
    where
        Self: 'a;
    fn write(&self, request: super::SessionWriteRequest) -> Self::WriteFuture<'_>;
}
