use std::{future::Future, pin::Pin};

use tokio_util::sync::CancellationToken;

use crate::{
    HostError, HostErrorCode, StreamClient, StreamFailure, StreamOperation, StreamPayload,
    StreamRead, StreamRequest, StreamResponse,
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

        let stream = match &request.operation {
            StreamOperation::Read { stream, .. }
            | StreamOperation::ReadUntilLimit { stream, .. }
            | StreamOperation::WriteAll { stream, .. }
            | StreamOperation::Flush { stream }
            | StreamOperation::Close { stream } => stream,
        };
        let timeout_ms = match &request.operation {
            StreamOperation::Read { timeout_ms, .. }
            | StreamOperation::ReadUntilLimit { timeout_ms, .. } => *timeout_ms,
            _ => None,
        };
        let deadlines = OperationDeadlines::new(&request.budget, timeout_ms);
        let Some(slot) = self.streams.stream_slot(stream.handle()).await else {
            return Ok(StreamResponse {
                id: request.id,
                result: Err(StreamFailure::Host(unknown_stream(stream.handle()))),
            });
        };
        let mut state = match lock_stream_state(&slot, stream.handle(), deadlines.clone()).await {
            Ok(state) => state,
            Err(error) => {
                return Ok(StreamResponse {
                    id: request.id,
                    result: Err(StreamFailure::from_host(error)),
                });
            }
        };
        let super::store::ManagedStreamState::Open(connection) = &mut *state else {
            return Ok(StreamResponse {
                id: request.id,
                result: Err(StreamFailure::Closed),
            });
        };
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
            StreamOperation::WriteAll { stream: _, body } => {
                match await_io(
                    connection.write_all_and_flush(body),
                    Some(&slot.cancellation),
                    deadlines.clone(),
                    "stream write failed",
                )
                .await
                {
                    Ok(()) => Ok(StreamPayload::Unit),
                    Err(error) => Err(StreamFailure::from_host(error)),
                }
            }
            StreamOperation::Flush { stream: _ } => match await_io(
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
                    connection.prepend_read_buffer(body);
                    return Err(StreamFailure::LimitExceeded { limit_bytes });
                }
                body.extend_from_slice(&chunk[..read]);
            }
            Err(error) => {
                connection.prepend_read_buffer(body);
                return Err(StreamFailure::from_host(error));
            }
        }
    }
}
