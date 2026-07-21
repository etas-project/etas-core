use std::future::Future;

use super::{ConsoleRequest, ConsoleResponse};

pub trait ConsoleClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<ConsoleResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: ConsoleRequest) -> Self::ExecuteFuture<'_>;
}
