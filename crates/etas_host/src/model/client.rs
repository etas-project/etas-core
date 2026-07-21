use std::future::Future;

use crate::{ModelRequest, ModelResponse};

pub trait ModelClient {
    type Error;
    type CompleteFuture<'a>: Future<Output = Result<ModelResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn complete(&self, request: ModelRequest) -> Self::CompleteFuture<'_>;
}
