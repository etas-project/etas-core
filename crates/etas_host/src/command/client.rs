use std::future::Future;

use crate::{CommandRequest, CommandResponse};

pub trait CommandClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<CommandResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: CommandRequest) -> Self::ExecuteFuture<'_>;
}
