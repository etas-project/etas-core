use std::future::Future;

use crate::{ToolRequest, ToolResponse};

pub trait ToolClient {
    type Error;
    type InvokeFuture<'a>: Future<Output = Result<ToolResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn invoke(&self, request: ToolRequest) -> Self::InvokeFuture<'_>;
}
