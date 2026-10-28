use std::future::Future;

use crate::{
    HostError,
    compiler::{CompilerRequest, CompilerResponse},
    execution::OperationContext,
};

/// The caller owns the operation scope. A future's drop is not completion of
/// admitted compiler work; adapters retain its registration until work settles.
pub trait CompilerClient {
    type ExecuteFuture<'a>: Future<Output = Result<CompilerResponse, HostError>> + Send + 'a
    where
        Self: 'a;
    fn execute<'a>(
        &'a self,
        request: CompilerRequest,
        operation: &'a OperationContext,
    ) -> Self::ExecuteFuture<'a>;
}
