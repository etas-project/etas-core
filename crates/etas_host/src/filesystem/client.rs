use std::future::Future;

use crate::{FilesystemRequest, FilesystemResponse};

pub trait FilesystemClient {
    type Error;
    type ExecuteFuture<'a>: Future<Output = Result<FilesystemResponse, Self::Error>> + Send + 'a
    where
        Self: 'a;

    fn execute(&self, request: FilesystemRequest) -> Self::ExecuteFuture<'_>;
}
