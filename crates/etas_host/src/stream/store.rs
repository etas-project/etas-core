use std::{
    collections::HashMap,
    future::{Future, pending},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{Mutex, MutexGuard, RwLock},
    time::{Instant, sleep_until},
};
use tokio_rustls::client::TlsStream;
use tokio_util::sync::CancellationToken;

use crate::{HostError, HostErrorCode};

#[derive(Clone, Default)]
pub struct ByteStreamStore {
    next_stream_id: Arc<AtomicU64>,
    streams: Arc<RwLock<HashMap<String, Arc<StreamSlot>>>>,
}

pub(crate) struct StreamSlot {
    pub(crate) state: Mutex<Option<ManagedStream>>,
    pub(crate) cancellation: CancellationToken,
}

pub(crate) enum ManagedStream {
    Tcp(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl ManagedStream {
    pub(crate) async fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ManagedStream::Tcp(stream) => stream.read(buffer).await,
            ManagedStream::Tls(stream) => stream.read(buffer).await,
        }
    }

    pub(crate) async fn write_all_and_flush(&mut self, body: &[u8]) -> std::io::Result<()> {
        match self {
            ManagedStream::Tcp(stream) => {
                stream.write_all(body).await?;
                stream.flush().await
            }
            ManagedStream::Tls(stream) => {
                stream.write_all(body).await?;
                stream.flush().await
            }
        }
    }

    pub(crate) async fn flush(&mut self) -> std::io::Result<()> {
        match self {
            ManagedStream::Tcp(stream) => stream.flush().await,
            ManagedStream::Tls(stream) => stream.flush().await,
        }
    }
}

impl StreamSlot {
    fn new(stream: ManagedStream) -> Self {
        Self {
            state: Mutex::new(Some(stream)),
            cancellation: CancellationToken::new(),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OperationDeadlines {
    budget: Option<Instant>,
    operation: Option<Instant>,
    operation_timeout_ms: Option<u64>,
}

impl OperationDeadlines {
    pub(crate) fn new(budget: &crate::Budget, operation_timeout_ms: Option<u64>) -> Self {
        let now = Instant::now();
        Self {
            budget: budget
                .time
                .map(|budget| now + Duration::from_millis(budget.max_millis)),
            operation: operation_timeout_ms.map(|timeout| now + Duration::from_millis(timeout)),
            operation_timeout_ms,
        }
    }
}

impl ByteStreamStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn insert_stream(&self, id: String, stream: ManagedStream) {
        self.insert_slot(id, Arc::new(StreamSlot::new(stream)))
            .await;
    }

    pub(crate) async fn insert_slot(&self, id: String, slot: Arc<StreamSlot>) {
        self.streams.write().await.insert(id, slot);
    }

    pub(crate) async fn stream_slot(&self, id: &str) -> Option<Arc<StreamSlot>> {
        self.streams.read().await.get(id).cloned()
    }

    pub(crate) async fn remove_stream(&self, id: &str) -> Option<Arc<StreamSlot>> {
        self.streams.write().await.remove(id)
    }

    pub(crate) fn next_stream_id(&self) -> u64 {
        self.next_stream_id.fetch_add(1, Ordering::Relaxed)
    }
}

pub(crate) fn unknown_stream(id: &str) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "unknown byte stream")
        .with_detail("stream", id.to_owned())
}

pub(crate) fn stream_io_error(message: &'static str, error: std::io::Error) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, message)
        .with_detail("error", error.to_string())
}

pub(crate) async fn lock_stream_state<'a>(
    slot: &'a StreamSlot,
    deadlines: OperationDeadlines,
) -> Result<MutexGuard<'a, Option<ManagedStream>>, HostError> {
    check_expired_deadlines(deadlines)?;
    tokio::select! {
        biased;
        _ = slot.cancellation.cancelled() => Err(stream_cancelled()),
        _ = wait_until(deadlines.budget) => Err(time_budget_exceeded()),
        _ = wait_until(deadlines.operation) => Err(stream_timeout(deadlines.operation_timeout_ms)),
        state = slot.state.lock() => Ok(state),
    }
}

pub(crate) async fn await_io<T>(
    operation: impl Future<Output = std::io::Result<T>>,
    cancellation: Option<&CancellationToken>,
    deadlines: OperationDeadlines,
    io_message: &'static str,
) -> Result<T, HostError> {
    check_expired_deadlines(deadlines)?;
    tokio::select! {
        biased;
        _ = wait_for_cancellation(cancellation) => Err(stream_cancelled()),
        _ = wait_until(deadlines.budget) => Err(time_budget_exceeded()),
        _ = wait_until(deadlines.operation) => Err(stream_timeout(deadlines.operation_timeout_ms)),
        result = operation => result.map_err(|error| stream_io_error(io_message, error)),
    }
}

fn check_expired_deadlines(deadlines: OperationDeadlines) -> Result<(), HostError> {
    let now = Instant::now();
    if deadlines.budget.is_some_and(|deadline| deadline <= now) {
        return Err(time_budget_exceeded());
    }
    if deadlines.operation.is_some_and(|deadline| deadline <= now) {
        return Err(stream_timeout(deadlines.operation_timeout_ms));
    }
    Ok(())
}

async fn wait_for_cancellation(cancellation: Option<&CancellationToken>) {
    match cancellation {
        Some(cancellation) => cancellation.cancelled().await,
        None => pending::<()>().await,
    }
}

async fn wait_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => pending::<()>().await,
    }
}

fn stream_cancelled() -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "byte stream operation was cancelled",
    )
}

fn time_budget_exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "host operation exceeded its time budget",
    )
}

fn stream_timeout(timeout_ms: Option<u64>) -> HostError {
    HostError::new(
        HostErrorCode::ProviderUnavailable,
        "byte stream operation timed out",
    )
    .with_detail("timeout_ms", timeout_ms.unwrap_or_default().to_string())
}

#[cfg(test)]
mod tests {
    use std::{panic::AssertUnwindSafe, time::Duration};

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        time::timeout,
    };
    use tokio_util::sync::CancellationToken;

    use super::{ByteStreamStore, ManagedStream, OperationDeadlines};
    use crate::stream::local::read_until_limit;
    use crate::tls::local::tls_client_config;
    use crate::{
        AuthorityContext, Budget, ByteStreamRef, CommandPolicy, DestructiveOpPolicy,
        FilesystemPolicy, HostErrorCode, HostRequestId, LocalStreamClient, LocalTcpClient,
        LocalTlsClient, NetworkEndpoint, NetworkPolicy, SandboxPolicy, StreamClient,
        StreamOperation, StreamRequest, TcpClient, TcpConnectOperation, TcpConnectRequest,
        TcpEndpoint, TimeBudget, TlsClient, TlsConnectOperation, TlsConnectRequest, TraceContext,
        TraceId,
    };
    use crate::{StreamPayload, StreamRead};

    #[test]
    fn tls_client_config_builder_does_not_panic() {
        let result = std::panic::catch_unwind(AssertUnwindSafe(tls_client_config));
        assert!(result.is_ok(), "TLS client config construction panicked");
        result.expect("panic checked above");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tls_connect_to_plain_tcp_server_returns_host_error_without_panic() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept client");
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n").await;
        });
        let streams = ByteStreamStore::new();
        let tcp_client = LocalTcpClient::new(streams.clone());
        let tls_client = LocalTlsClient::new(streams);
        let authority = allow_network("127.0.0.1", address.port());

        let tcp = tcp_client
            .execute(TcpConnectRequest {
                id: HostRequestId(1),
                operation: TcpConnectOperation::Connect {
                    endpoint: TcpEndpoint {
                        host: "127.0.0.1".to_owned(),
                        port: address.port(),
                    },
                },
                authority: authority.clone(),
                trace: TraceContext::root(TraceId(1)),
                budget: Budget::default(),
            })
            .await
            .expect("host execution should not fail")
            .result
            .expect("tcp connect should succeed");

        let tls = tls_client
            .execute(TlsConnectRequest {
                id: HostRequestId(2),
                operation: TlsConnectOperation::Connect {
                    stream: tcp,
                    server_name: "127.0.0.1".to_owned(),
                },
                authority,
                trace: TraceContext::root(TraceId(1)),
                budget: Budget::default(),
            })
            .await
            .expect("host execution should not fail");

        server.await.expect("server should finish");
        let error = tls.result.expect_err("plain TCP must not become TLS");
        assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
        assert!(
            error.message.contains("TLS"),
            "unexpected TLS error message: {}",
            error.message
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tcp_connect_rejects_allowlisted_alternative_loopback_encoding() {
        let client = LocalTcpClient::new(ByteStreamStore::new());
        let response = client
            .execute(TcpConnectRequest {
                id: HostRequestId(3),
                operation: TcpConnectOperation::Connect {
                    endpoint: TcpEndpoint {
                        host: "0x7f000001".to_owned(),
                        port: 8848,
                    },
                },
                authority: allow_network("0x7f000001", 8848),
                trace: TraceContext::root(TraceId(3)),
                budget: Budget::default(),
            })
            .await
            .expect("host execution should return a typed response");
        let error = response
            .result
            .expect_err("alternative loopback encoding must not reach the provider");
        assert_eq!(error.code, HostErrorCode::AuthorityDenied);
        assert!(error.message.contains("non-canonical IP"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_until_limit_accumulates_until_eof() {
        let (mut stream, mut writer) = connected_stream().await;
        writer
            .write_all(b"hello ")
            .await
            .expect("write first chunk");
        writer
            .write_all(b"world")
            .await
            .expect("write second chunk");
        writer.shutdown().await.expect("finish body");

        let cancellation = CancellationToken::new();
        let payload = read_until_limit(
            &mut stream,
            32,
            &cancellation,
            OperationDeadlines::new(&Budget::default(), Some(2_000)),
        )
        .await
        .expect("read should succeed");
        assert_eq!(
            payload,
            StreamPayload::Read(StreamRead::Data(b"hello world".to_vec()))
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn read_until_limit_fails_when_body_exceeds_limit() {
        let (mut stream, mut writer) = connected_stream().await;
        writer.write_all(b"abc").await.expect("write body");
        writer.shutdown().await.expect("finish body");

        let cancellation = CancellationToken::new();
        let error = read_until_limit(
            &mut stream,
            2,
            &cancellation,
            OperationDeadlines::new(&Budget::default(), Some(2_000)),
        )
        .await
        .expect_err("limit must fail");
        assert_eq!(error.code, crate::HostErrorCode::BudgetExceeded);
        assert!(
            error.message.contains("exceeded byte limit"),
            "{}",
            error.message
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocked_read_does_not_block_other_stream_and_close_interrupts_it() {
        let streams = ByteStreamStore::new();
        let stream_client = LocalStreamClient::new(streams.clone());
        let (blocked_stream, _blocked_peer) = connected_stream().await;
        let (writable_stream, mut writable_peer) = connected_stream().await;
        streams
            .insert_stream("blocked".to_owned(), blocked_stream)
            .await;
        streams
            .insert_stream("writable".to_owned(), writable_stream)
            .await;

        let read_client = stream_client.clone();
        let reader = tokio::spawn(async move {
            read_client
                .execute(stream_request(
                    10,
                    StreamOperation::Read {
                        stream: ByteStreamRef::opaque("blocked"),
                        max_bytes: 1,
                        timeout_ms: None,
                    },
                ))
                .await
        });
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(25)).await;

        let write = timeout(
            Duration::from_secs(1),
            stream_client.execute(stream_request(
                11,
                StreamOperation::WriteAll {
                    stream: ByteStreamRef::opaque("writable"),
                    body: b"ok".to_vec(),
                },
            )),
        )
        .await
        .expect("blocked read must not stall another stream")
        .expect("second stream write should execute");
        assert_eq!(write.result, Ok(StreamPayload::Unit));
        let mut received = [0_u8; 2];
        timeout(
            Duration::from_secs(1),
            writable_peer.read_exact(&mut received),
        )
        .await
        .expect("peer read must not be stalled")
        .expect("peer should receive second stream write");
        assert_eq!(&received, b"ok");

        let close = timeout(
            Duration::from_secs(1),
            stream_client.execute(stream_request(
                12,
                StreamOperation::Close {
                    stream: ByteStreamRef::opaque("blocked"),
                },
            )),
        )
        .await
        .expect("close must not wait for the blocked read")
        .expect("close should execute without acquiring the blocked stream state lock");
        assert_eq!(close.result, Ok(StreamPayload::Unit));
        let read = timeout(Duration::from_secs(1), reader)
            .await
            .expect("close should cancel the blocked read")
            .expect("read task should not panic")
            .expect("blocked read host execution should complete");
        let error = read
            .result
            .expect_err("cancelled read must fail explicitly");
        assert_eq!(error.code, HostErrorCode::ProviderUnavailable);
        assert!(error.message.contains("cancelled"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tcp_connect_consumes_time_budget() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let response = LocalTcpClient::new(ByteStreamStore::new())
            .execute(TcpConnectRequest {
                id: HostRequestId(40),
                operation: TcpConnectOperation::Connect {
                    endpoint: TcpEndpoint {
                        host: "127.0.0.1".to_owned(),
                        port: address.port(),
                    },
                },
                authority: allow_network("127.0.0.1", address.port()),
                trace: TraceContext::root(TraceId(40)),
                budget: Budget {
                    time: Some(TimeBudget { max_millis: 0 }),
                    ..Budget::default()
                },
            })
            .await
            .expect("host execution should return a typed response");
        let error = response
            .result
            .expect_err("zero time budget must prevent TCP connect");
        assert_eq!(error.code, HostErrorCode::BudgetExceeded);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tls_handshake_consumes_time_budget() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("accept client");
            tokio::time::sleep(Duration::from_secs(1)).await;
        });
        let streams = ByteStreamStore::new();
        let tcp_client = LocalTcpClient::new(streams.clone());
        let tls_client = LocalTlsClient::new(streams);
        let authority = allow_network("127.0.0.1", address.port());
        let tcp = tcp_client
            .execute(TcpConnectRequest {
                id: HostRequestId(41),
                operation: TcpConnectOperation::Connect {
                    endpoint: TcpEndpoint {
                        host: "127.0.0.1".to_owned(),
                        port: address.port(),
                    },
                },
                authority: authority.clone(),
                trace: TraceContext::root(TraceId(41)),
                budget: Budget {
                    time: Some(TimeBudget { max_millis: 500 }),
                    ..Budget::default()
                },
            })
            .await
            .expect("host execution should succeed")
            .result
            .expect("TCP connect should succeed");
        let response = tls_client
            .execute(TlsConnectRequest {
                id: HostRequestId(42),
                operation: TlsConnectOperation::Connect {
                    stream: tcp,
                    server_name: "127.0.0.1".to_owned(),
                },
                authority,
                trace: TraceContext::root(TraceId(42)),
                budget: Budget {
                    time: Some(TimeBudget { max_millis: 25 }),
                    ..Budget::default()
                },
            })
            .await
            .expect("host execution should return a typed response");
        let error = response
            .result
            .expect_err("stalled TLS handshake must exhaust time budget");
        assert_eq!(error.code, HostErrorCode::BudgetExceeded);
        server.abort();
    }

    async fn connected_stream() -> (ManagedStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let (client, accepted) = tokio::join!(TcpStream::connect(address), listener.accept());
        let client = client.expect("connect client");
        let (server, _) = accepted.expect("accept client");
        (ManagedStream::Tcp(client), server)
    }

    fn stream_request(id: u32, operation: StreamOperation) -> StreamRequest {
        StreamRequest {
            id: HostRequestId(id),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(id)),
            budget: Budget::default(),
        }
    }

    fn allow_network(host: &str, port: u16) -> AuthorityContext {
        AuthorityContext {
            grants: Vec::new(),
            approvals: Vec::new(),
            sandbox: SandboxPolicy::allow_listed(
                FilesystemPolicy::deny_all(),
                NetworkPolicy::allow_endpoints(vec![
                    NetworkEndpoint::new("tcp", host, port),
                    NetworkEndpoint::new("tls", host, port),
                ]),
                CommandPolicy::deny_all(),
                DestructiveOpPolicy::deny_all(),
            ),
            policy: Default::default(),
        }
    }
}
