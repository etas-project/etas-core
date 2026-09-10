use std::future::Future;

use super::{MemoryWriteRequest, MemoryWriteResponse};
use crate::{MemoryRequest, MemoryResponse};

pub trait MemoryClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<MemoryResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: MemoryRequest) -> Self::ExecuteFuture<'_>;
    type WriteFuture<'a>: Future<Output = Result<MemoryWriteResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;
    fn write(&self, request: MemoryWriteRequest) -> Self::WriteFuture<'_>;
}
