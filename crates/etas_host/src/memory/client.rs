use std::future::Future;

use crate::{MemoryRequest, MemoryResponse};

pub trait MemoryClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<MemoryResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: MemoryRequest) -> Self::ExecuteFuture<'_>;
}
