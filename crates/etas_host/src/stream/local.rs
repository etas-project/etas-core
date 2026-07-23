use std::{future::Future, pin::Pin};

use tokio_util::sync::CancellationToken;

use crate::{
    HostError, HostErrorCode, StreamClient, StreamOperation, StreamPayload, StreamRead,
    StreamRequest, StreamResponse,
};

use super::{
    ByteStreamStore,
    store::{ManagedStream, OperationDeadlines, await_io, lock_stream_state, unknown_stream},
};

#[derive(Clone, Default)]
pub struct LocalStreamClient {
    streams: ByteStreamStore,
}

impl LocalStreamClient {
    pub fn new(streams: ByteStreamStore) -> Self {
        Self { streams }
    }

    async fn execute_request(&self, request: StreamRequest) -> Result<StreamResponse, HostError> {
        if let StreamOperation::Close { stream } = &request.operation {
            if let Some(slot) = self.streams.remove_stream(&stream.id).await {
                slot.cancellation.cancel();
            }
            return Ok(StreamResponse {
                id: request.id,
                result: Ok(StreamPayload::Unit),
            });
        }

        let stream_id = match &request.operation {
            StreamOperation::Read { stream, .. }
            | StreamOperation::ReadUntilLimit { stream, .. }
            | StreamOperation::WriteAll { stream, .. }
            | StreamOperation::Flush { stream }
            | StreamOperation::Close { stream } => &stream.id,
        };
        let timeout_ms = match &request.operation {
            StreamOperation::Read { timeout_ms, .. }
            | StreamOperation::ReadUntilLimit { timeout_ms, .. } => *timeout_ms,
            _ => None,
        };
        let deadlines = OperationDeadlines::new(&request.budget, timeout_ms);
        let Some(slot) = self.streams.stream_slot(stream_id).await else {
            return Ok(StreamResponse {
                id: request.id,
                result: Err(unknown_stream(stream_id)),
            });
        };
        let mut state = match lock_stream_state(&slot, deadlines).await {
            Ok(state) => state,
            Err(error) => {
                return Ok(StreamResponse {
                    id: request.id,
                    result: Err(error),
                });
            }
        };
        let Some(connection) = state.as_mut() else {
            return Ok(StreamResponse {
                id: request.id,
                result: Err(unknown_stream(stream_id)),
            });
        };
        let response = match &request.operation {
            StreamOperation::Read {
                stream: _,
                max_bytes,
                timeout_ms: _,
            } => {
                let mut buffer = vec![0; *max_bytes];
                match await_io(
                    connection.read(&mut buffer),
                    Some(&slot.cancellation),
                    deadlines,
                    "stream read failed",
                )
                .await
                {
                    Ok(0) => Ok(StreamPayload::Read(StreamRead::Eof)),
                    Ok(read) => {
                        buffer.truncate(read);
                        Ok(StreamPayload::Read(StreamRead::Data(buffer)))
                    }
                    Err(error) => Err(error),
                }
            }
            StreamOperation::ReadUntilLimit {
                stream: _,
                limit_bytes,
                timeout_ms: _,
            } => read_until_limit(connection, *limit_bytes, &slot.cancellation, deadlines).await,
            StreamOperation::WriteAll { stream: _, body } => {
                match await_io(
                    connection.write_all_and_flush(body),
                    Some(&slot.cancellation),
                    deadlines,
                    "stream write failed",
                )
                .await
                {
                    Ok(()) => Ok(StreamPayload::Unit),
                    Err(error) => Err(error),
                }
            }
            StreamOperation::Flush { stream: _ } => match await_io(
                connection.flush(),
                Some(&slot.cancellation),
                deadlines,
                "stream flush failed",
            )
            .await
            {
                Ok(()) => Ok(StreamPayload::Unit),
                Err(error) => Err(error),
            },
            StreamOperation::Close { .. } => Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "stream close dispatch invariant violated",
            )),
        };
        Ok(StreamResponse {
            id: request.id,
            result: response,
        })
    }
}

impl StreamClient for LocalStreamClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<StreamResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: StreamRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.execute_request(request))
    }
}

pub(super) async fn read_until_limit(
    connection: &mut ManagedStream,
    limit_bytes: usize,
    cancellation: &CancellationToken,
    deadlines: OperationDeadlines,
) -> Result<StreamPayload, HostError> {
    let mut body = Vec::new();

    loop {
        let remaining = limit_bytes.saturating_sub(body.len());
        let chunk_size = if remaining == 0 {
            1
        } else {
            remaining.min(8 * 1024)
        };
        let mut chunk = vec![0; chunk_size];
        match await_io(
            connection.read(&mut chunk),
            Some(cancellation),
            deadlines,
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
                    return Err(HostError::new(
                        HostErrorCode::BudgetExceeded,
                        "stream read exceeded byte limit before EOF",
                    )
                    .with_detail("limit_bytes", limit_bytes.to_string()));
                }
                body.extend_from_slice(&chunk[..read]);
            }
            Err(error) => return Err(error),
        }
    }
}
