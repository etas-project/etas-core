use std::{future::Future, pin::Pin};

use tokio_util::sync::CancellationToken;

use crate::{
    HostError, HostErrorCode, StreamClient, StreamFailure, StreamOperation, StreamPayload,
    StreamRead, StreamRequest, StreamResponse,
};

use super::{
    ByteStreamStore,
    admission::StreamAccess,
    store::{ManagedStream, OperationDeadlines, admit_stream, await_io, unknown_stream},
};

#[derive(Clone, Default)]
pub struct LocalStreamClient {
    streams: ByteStreamStore,
}

impl LocalStreamClient {
    pub fn new(streams: ByteStreamStore) -> Self {
        Self { streams }
    }

    async fn execute_request(
        &self,
        request: StreamRequest,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<StreamResponse, HostError> {
        if let StreamOperation::Close { stream } = &request.operation {
            let result = match self.streams.stream_slot(stream.handle()).await {
                Some(slot) => match slot.begin_close(stream.handle()) {
                    Ok(()) => {
                        slot.finish_close().await;
                        Ok(StreamPayload::Unit)
                    }
                    Err(error) => Err(StreamFailure::from_host(error)),
                },
                None => Err(StreamFailure::Host(unknown_stream(stream.handle()))),
            };
            return Ok(StreamResponse {
                id: request.id,
                result,
            });
        }

        if let Some(operation) = operation {
            operation.signal().check()?;
        }

        let stream = match &request.operation {
            StreamOperation::Read { stream, .. }
            | StreamOperation::ReadUntilLimit { stream, .. }
            | StreamOperation::WriteAll { stream, .. }
            | StreamOperation::Flush { stream, .. }
            | StreamOperation::Close { stream } => stream,
        };
        let timeout_ms = match &request.operation {
            StreamOperation::Read { timeout_ms, .. }
            | StreamOperation::ReadUntilLimit { timeout_ms, .. }
            | StreamOperation::WriteAll { timeout_ms, .. }
            | StreamOperation::Flush { timeout_ms, .. } => *timeout_ms,
            _ => None,
        };
        let deadlines =
            OperationDeadlines::new(&request.budget, timeout_ms).with_operation(operation);
        let Some(slot) = self.streams.stream_slot(stream.handle()).await else {
            return Ok(StreamResponse {
                id: request.id,
                result: Err(StreamFailure::Host(unknown_stream(stream.handle()))),
            });
        };
        let access = match request.operation {
            StreamOperation::Read { .. } | StreamOperation::ReadUntilLimit { .. } => {
                StreamAccess::Read
            }
            StreamOperation::WriteAll { .. } | StreamOperation::Flush { .. } => StreamAccess::Write,
            StreamOperation::Close { .. } => unreachable!("close bypasses I/O admission"),
        };
        let scope = if matches!(access, StreamAccess::Read) {
            stream.read_scope.as_ref().and_then(|scope| scope.upgrade())
        } else {
            None
        };
        let admission = match scope {
            Some(scope) => scope.admit(&slot, stream.handle(), deadlines.clone()).await,
            None => admit_stream(&slot, stream.handle(), access, deadlines.clone()).await,
        };
        let admitted = match admission {
            Ok(admitted) => admitted,
            Err(error) => {
                return Ok(StreamResponse {
                    id: request.id,
                    result: Err(StreamFailure::from_host(error)),
                });
            }
        };
        let connection = &admitted.stream;
        if let Err(error) = connection.validate_access(matches!(access, StreamAccess::Write)) {
            return Ok(StreamResponse {
                id: request.id,
                result: Err(StreamFailure::from_host(error)),
            });
        }
        if let crate::ByteStreamOrigin::File { region, .. } = stream.origin() {
            let right = if matches!(access, StreamAccess::Read) {
                "read"
            } else {
                "write"
            };
            let action = crate::ActionInstance::new(
                "Fs",
                right,
                vec![crate::HostValue::String(region.as_str().into())],
            );
            if !request.authority.allows(&action) {
                return Err(HostError::new(
                    HostErrorCode::AuthorityDenied,
                    "file stream operation lacks its region authority",
                ));
            }
        }
        let response = match &request.operation {
            StreamOperation::Read {
                stream: _,
                max_bytes,
                timeout_ms: _,
            } => {
                if *max_bytes == 0 {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Ok(StreamPayload::Read(StreamRead::Data(Vec::new()))),
                    });
                }
                let mut buffer = vec![0; *max_bytes];
                match await_io(
                    connection.read(&mut buffer),
                    Some(&slot.cancellation),
                    deadlines.clone(),
                    "stream read failed",
                )
                .await
                {
                    Ok(0) => Ok(StreamPayload::Read(StreamRead::Eof)),
                    Ok(read) => {
                        buffer.truncate(read);
                        Ok(StreamPayload::Read(StreamRead::Data(buffer)))
                    }
                    Err(error) => Err(StreamFailure::from_host(error)),
                }
            }
            StreamOperation::ReadUntilLimit {
                stream: _,
                limit_bytes,
                timeout_ms: _,
            } => read_until_limit(connection, *limit_bytes, &slot.cancellation, deadlines).await,
            StreamOperation::WriteAll { body, .. } => {
                write_body(
                    connection,
                    body,
                    &slot.cancellation,
                    deadlines.clone(),
                    operation,
                )
                .await
            }
            StreamOperation::Flush { .. } => match await_io(
                connection.flush(),
                Some(&slot.cancellation),
                deadlines.clone(),
                "stream flush failed",
            )
            .await
            {
                Ok(()) => Ok(StreamPayload::Unit),
                Err(error) => Err(StreamFailure::from_host(error)),
            },
            StreamOperation::Close { .. } => Err(StreamFailure::Host(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "stream close dispatch invariant violated",
            ))),
        };
        // A partially written frame cannot be reused. Read cancellation preserves
        // buffered input and does not close a stream shared with other requests.
        if response.is_err() && matches!(request.operation, StreamOperation::WriteAll { .. }) {
            drop(admitted);
            match slot.begin_close(stream.handle()) {
                Ok(()) => slot.finish_close().await,
                Err(error) if error.code == HostErrorCode::Closed => slot.finish_close().await,
                Err(error) => return Err(error),
            }
        }
        Ok(StreamResponse {
            id: request.id,
            result: response,
        })
    }

    pub async fn execute_scoped(
        &self,
        request: StreamRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<StreamResponse, HostError> {
        let client = self.clone();
        operation
            .supervise(move |context| async move {
                client.execute_request(request, Some(&context)).await
            })
            .await
    }
}

impl StreamClient for LocalStreamClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<StreamResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: StreamRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.execute_request(request, None))
    }
}

async fn write_body(
    connection: &ManagedStream,
    body: &[u8],
    cancellation: &CancellationToken,
    deadlines: OperationDeadlines,
    operation: Option<&crate::execution::OperationContext>,
) -> Result<StreamPayload, StreamFailure> {
    let mut written = 0;
    while written < body.len() {
        let count = await_io(
            connection.write(&body[written..]),
            Some(cancellation),
            deadlines.clone(),
            "stream write failed",
        )
        .await
        .map_err(StreamFailure::from_host)?;
        if count == 0 {
            return Err(StreamFailure::Closed);
        }
        written += count;
        if let Some(operation) = operation {
            operation
                .record_progress(written as u64)
                .map_err(StreamFailure::from_host)?;
        }
    }
    await_io(
        connection.flush(),
        Some(cancellation),
        deadlines,
        "stream flush failed",
    )
    .await
    .map_err(StreamFailure::from_host)?;
    Ok(StreamPayload::Unit)
}

pub(super) async fn read_until_limit(
    connection: &ManagedStream,
    limit_bytes: usize,
    cancellation: &CancellationToken,
    deadlines: OperationDeadlines,
) -> Result<StreamPayload, StreamFailure> {
    let mut body = Vec::new();

    if limit_bytes == 0 {
        return Ok(StreamPayload::Read(StreamRead::Data(body)));
    }

    loop {
        let remaining = limit_bytes.saturating_sub(body.len());
        if remaining == 0 {
            return Ok(StreamPayload::Read(StreamRead::Data(body)));
        }
        let chunk_size = remaining.saturating_add(1).min(8 * 1024);
        let mut chunk = vec![0; chunk_size];
        match await_io(
            connection.read(&mut chunk),
            Some(cancellation),
            deadlines.clone(),
            "stream read-until-limit failed",
        )
        .await
        {
            Ok(0) => {
                return if body.is_empty() {
                    Ok(StreamPayload::Read(StreamRead::Eof))
                } else {
                    Ok(StreamPayload::Read(StreamRead::Data(body)))
                };
            }
            Ok(read) => {
                if body.len().saturating_add(read) > limit_bytes {
                    body.extend_from_slice(&chunk[..read]);
                    connection.prepend_read_buffer(body).await;
                    return Err(StreamFailure::LimitExceeded { limit_bytes });
                }
                body.extend_from_slice(&chunk[..read]);
            }
            Err(error) => {
                connection.prepend_read_buffer(body).await;
                return Err(StreamFailure::from_host(error));
            }
        }
    }
}
