use std::future::Future;

use crate::{CommandProcessRequest, CommandProcessResponse, CommandRequest, CommandResponse};

pub trait CommandClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<CommandResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: CommandRequest) -> Self::ExecuteFuture<'_>;
}

pub trait CommandProcessClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<CommandProcessResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute_process(&self, request: CommandProcessRequest) -> Self::ExecuteFuture<'_>;
}
