use std::{
    collections::{HashMap, VecDeque, hash_map::Entry},
    fmt::Write,
    future::{Future, pending},
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU64, Ordering},
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

use crate::{ByteStreamOrigin, HostError, HostErrorCode, StreamHandleRef};

#[derive(Clone, Default)]
pub struct ByteStreamStore {
    streams: Arc<RwLock<HashMap<String, Arc<StreamSlot>>>>,
}

pub(crate) struct StreamSlot {
    state: Mutex<ManagedStreamState>,
    lifecycle: AtomicU8,
    generation: AtomicU64,
    pub(crate) cancellation: CancellationToken,
}

pub(crate) enum ManagedStreamState {
    Open {
        stream: ManagedStream,
        origin: ByteStreamOrigin,
    },
    Upgrading,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum StreamLifecycle {
    OpenTcp = 0,
    Upgrading = 1,
    OpenTls = 2,
    Closing = 3,
    Closed = 4,
}

pub(crate) enum ManagedStream {
    Tcp {
        stream: TcpStream,
        read_buffer: VecDeque<u8>,
    },
    Tls {
        stream: Box<TlsStream<TcpStream>>,
        read_buffer: VecDeque<u8>,
    },
}

impl ManagedStream {
    pub(crate) fn tcp(stream: TcpStream) -> Self {
        Self::Tcp {
            stream,
            read_buffer: VecDeque::new(),
        }
    }

    pub(crate) fn tls(stream: TlsStream<TcpStream>) -> Self {
        Self::Tls {
            stream: Box::new(stream),
            read_buffer: VecDeque::new(),
        }
    }

    pub(crate) async fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let buffered = self.read_from_buffer(buffer);
        if buffered != 0 {
            return Ok(buffered);
        }
        match self {
            ManagedStream::Tcp { stream, .. } => stream.read(buffer).await,
            ManagedStream::Tls { stream, .. } => stream.read(buffer).await,
        }
    }

    pub(crate) fn prepend_read_buffer(&mut self, bytes: Vec<u8>) {
        let read_buffer = match self {
            Self::Tcp { read_buffer, .. } | Self::Tls { read_buffer, .. } => read_buffer,
        };
        for byte in bytes.into_iter().rev() {
            read_buffer.push_front(byte);
        }
    }

    fn read_from_buffer(&mut self, output: &mut [u8]) -> usize {
        let read_buffer = match self {
            Self::Tcp { read_buffer, .. } | Self::Tls { read_buffer, .. } => read_buffer,
        };
        let requested = output.len().min(read_buffer.len());
        let mut read = 0;
        for output in &mut output[..requested] {
            let Some(byte) = read_buffer.pop_front() else {
                break;
            };
            *output = byte;
            read += 1;
        }
        read
    }

    fn is_plain_tcp_without_buffered_data(&self) -> bool {
        matches!(
            self,
            Self::Tcp {
                read_buffer,
                ..
            } if read_buffer.is_empty()
        )
    }

    pub(crate) async fn write_all_and_flush(&mut self, body: &[u8]) -> std::io::Result<()> {
        match self {
            ManagedStream::Tcp { stream, .. } => {
                stream.write_all(body).await?;
                stream.flush().await
            }
            ManagedStream::Tls { stream, .. } => {
                stream.write_all(body).await?;
                stream.flush().await
            }
        }
    }

    pub(crate) async fn flush(&mut self) -> std::io::Result<()> {
        match self {
            ManagedStream::Tcp { stream, .. } => stream.flush().await,
            ManagedStream::Tls { stream, .. } => stream.flush().await,
        }
    }
}

impl StreamSlot {
    fn new(stream: ManagedStream, origin: ByteStreamOrigin) -> Self {
        let lifecycle = match stream {
            ManagedStream::Tcp { .. } => StreamLifecycle::OpenTcp,
            ManagedStream::Tls { .. } => StreamLifecycle::OpenTls,
        };
        Self {
            state: Mutex::new(ManagedStreamState::Open { stream, origin }),
            lifecycle: AtomicU8::new(lifecycle as u8),
            generation: AtomicU64::new(0),
            cancellation: CancellationToken::new(),
        }
    }

    pub(crate) fn lifecycle(&self) -> StreamLifecycle {
        match self.lifecycle.load(Ordering::Acquire) {
            value if value == StreamLifecycle::OpenTcp as u8 => StreamLifecycle::OpenTcp,
            value if value == StreamLifecycle::Upgrading as u8 => StreamLifecycle::Upgrading,
            value if value == StreamLifecycle::OpenTls as u8 => StreamLifecycle::OpenTls,
            value if value == StreamLifecycle::Closing as u8 => StreamLifecycle::Closing,
            _ => StreamLifecycle::Closed,
        }
    }

    fn validate_generation(&self, handle: &StreamHandleRef) -> Result<(), HostError> {
        let actual = self.generation.load(Ordering::Acquire);
        if actual == handle.generation() {
            Ok(())
        } else {
            Err(stale_stream(handle, actual))
        }
    }

    pub(crate) fn begin_close(&self, handle: &StreamHandleRef) -> Result<(), HostError> {
        self.validate_generation(handle)?;
        loop {
            let lifecycle = self.lifecycle();
            match lifecycle {
                StreamLifecycle::OpenTcp
                | StreamLifecycle::Upgrading
                | StreamLifecycle::OpenTls => {
                    if self
                        .lifecycle
                        .compare_exchange(
                            lifecycle as u8,
                            StreamLifecycle::Closing as u8,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                    {
                        self.cancellation.cancel();
                        return Ok(());
                    }
                }
                StreamLifecycle::Closing | StreamLifecycle::Closed => {
                    return Err(stream_closed());
                }
            }
        }
    }

    pub(crate) async fn finish_close(&self) {
        let mut state = self.state.lock().await;
        *state = ManagedStreamState::Closed;
        self.lifecycle
            .store(StreamLifecycle::Closed as u8, Ordering::Release);
    }

    pub(crate) fn begin_tls_upgrade(
        &self,
        handle: &StreamHandleRef,
    ) -> Result<(tokio::net::TcpStream, ByteStreamOrigin), HostError> {
        self.validate_generation(handle)?;
        let mut state = self.state.try_lock().map_err(|_| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "TLS handshake requires an idle TCP stream",
            )
        })?;
        if self.lifecycle() != StreamLifecycle::OpenTcp {
            return Err(stream_closed());
        }
        let ManagedStreamState::Open {
            stream: managed, ..
        } = &*state
        else {
            return Err(stream_closed());
        };
        if !managed.is_plain_tcp_without_buffered_data() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "TLS handshake requires an unread plain TCP stream",
            ));
        }
        self.lifecycle
            .compare_exchange(
                StreamLifecycle::OpenTcp as u8,
                StreamLifecycle::Upgrading as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map_err(|_| stream_closed())?;
        let previous = std::mem::replace(&mut *state, ManagedStreamState::Upgrading);
        match previous {
            ManagedStreamState::Open {
                stream: ManagedStream::Tcp { stream, .. },
                origin,
            } => Ok((stream, origin)),
            other => {
                *state = other;
                self.lifecycle
                    .store(StreamLifecycle::Closed as u8, Ordering::Release);
                Err(HostError::new(
                    HostErrorCode::InvalidRequest,
                    "TCP stream state changed during TLS upgrade",
                ))
            }
        }
    }

    pub(crate) fn validate_tls_upgrade(
        &self,
        handle: &StreamHandleRef,
    ) -> Result<ByteStreamOrigin, HostError> {
        self.validate_generation(handle)?;
        let state = self.state.try_lock().map_err(|_| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "TLS handshake requires an idle TCP stream",
            )
        })?;
        if self.lifecycle() != StreamLifecycle::OpenTcp {
            return Err(stream_closed());
        }
        let ManagedStreamState::Open { stream, origin } = &*state else {
            return Err(stream_closed());
        };
        if !stream.is_plain_tcp_without_buffered_data() {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "TLS handshake requires an unread plain TCP stream",
            ));
        }
        Ok(origin.clone())
    }

    pub(crate) async fn finish_tls_upgrade(
        &self,
        stream: TlsStream<TcpStream>,
        origin: ByteStreamOrigin,
    ) -> Result<u64, HostError> {
        let mut state = self.state.lock().await;
        if self
            .lifecycle
            .compare_exchange(
                StreamLifecycle::Upgrading as u8,
                StreamLifecycle::OpenTls as u8,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            *state = ManagedStreamState::Closed;
            return Err(stream_cancelled());
        }
        *state = ManagedStreamState::Open {
            stream: ManagedStream::tls(stream),
            origin,
        };
        Ok(self.generation.fetch_add(1, Ordering::AcqRel) + 1)
    }

    pub(crate) async fn fail_tls_upgrade(&self) {
        let mut state = self.state.lock().await;
        *state = ManagedStreamState::Closed;
        if self.lifecycle() == StreamLifecycle::Upgrading {
            self.lifecycle
                .store(StreamLifecycle::Closed as u8, Ordering::Release);
        }
    }
}

#[derive(Clone)]
pub(crate) struct OperationDeadlines {
    budget: Option<Instant>,
    budget_error: Option<HostError>,
    operation: Option<Instant>,
    operation_deadline_valid: bool,
    operation_timeout_ms: Option<u64>,
}

impl OperationDeadlines {
    pub(crate) fn new(budget: &crate::ExecutionBudget, operation_timeout_ms: Option<u64>) -> Self {
        let now = Instant::now();
        let (budget, budget_error) = match budget.deadline() {
            Ok(deadline) => (deadline, None),
            Err(error) => (None, Some(error)),
        };
        let operation = operation_timeout_ms
            .and_then(|timeout| now.checked_add(Duration::from_millis(timeout)));
        Self {
            budget,
            budget_error,
            operation,
            operation_deadline_valid: operation_timeout_ms.is_none() || operation.is_some(),
            operation_timeout_ms,
        }
    }
}

impl ByteStreamStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn insert_stream(
        &self,
        stream: ManagedStream,
        origin: ByteStreamOrigin,
    ) -> Result<StreamHandleRef, HostError> {
        loop {
            let token = random_stream_token()?;
            let mut streams = self.streams.write().await;
            match streams.entry(token.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(Arc::new(StreamSlot::new(stream, origin)));
                    return Ok(StreamHandleRef::issued(token, 0));
                }
                Entry::Occupied(_) => continue,
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn insert_stream_for_test(
        &self,
        token: impl Into<String>,
        stream: ManagedStream,
    ) -> StreamHandleRef {
        let token = token.into();
        self.streams.write().await.insert(
            token.clone(),
            Arc::new(StreamSlot::new(stream, ByteStreamOrigin::Opaque)),
        );
        StreamHandleRef::issued(token, 0)
    }

    pub(crate) async fn stream_slot(&self, handle: &StreamHandleRef) -> Option<Arc<StreamSlot>> {
        self.streams.read().await.get(handle.token()).cloned()
    }
}

fn random_stream_token() -> Result<String, HostError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "failed to generate byte stream capability token",
        )
        .with_detail("error", error.to_string())
    })?;
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut token, "{byte:02x}").map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to encode byte stream capability token",
            )
            .with_detail("error", error.to_string())
        })?;
    }
    Ok(token)
}

pub(crate) fn unknown_stream(handle: &StreamHandleRef) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "unknown byte stream")
        .with_detail("stream", handle.identity_fingerprint())
}

fn stale_stream(handle: &StreamHandleRef, actual_generation: u64) -> HostError {
    HostError::new(
        HostErrorCode::Closed,
        "byte stream handle generation is stale",
    )
    .with_detail("stream", handle.identity_fingerprint())
    .with_detail("actual_generation", actual_generation.to_string())
}

pub(crate) fn stream_io_error(message: &'static str, error: std::io::Error) -> HostError {
    let code = match error.kind() {
        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => HostErrorCode::TimedOut,
        std::io::ErrorKind::Interrupted => HostErrorCode::Interrupted,
        std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::ConnectionAborted
        | std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::NotConnected
        | std::io::ErrorKind::UnexpectedEof
        | std::io::ErrorKind::WriteZero => HostErrorCode::Closed,
        _ => HostErrorCode::ProviderUnavailable,
    };
    HostError::new(code, message).with_detail("error", error.to_string())
}

pub(crate) async fn lock_stream_state<'a>(
    slot: &'a StreamSlot,
    handle: &StreamHandleRef,
    deadlines: OperationDeadlines,
) -> Result<MutexGuard<'a, ManagedStreamState>, HostError> {
    slot.validate_generation(handle)?;
    if !matches!(
        slot.lifecycle(),
        StreamLifecycle::OpenTcp | StreamLifecycle::OpenTls
    ) {
        return Err(stream_closed());
    }
    check_expired_deadlines(&deadlines)?;
    tokio::select! {
        biased;
        _ = slot.cancellation.cancelled() => Err(stream_cancelled()),
        _ = wait_until(deadlines.budget) => Err(time_budget_exceeded()),
        _ = wait_until(deadlines.operation) => Err(stream_timeout(deadlines.operation_timeout_ms)),
        state = slot.state.lock() => {
            slot.validate_generation(handle)?;
            if matches!(slot.lifecycle(), StreamLifecycle::OpenTcp | StreamLifecycle::OpenTls) {
                Ok(state)
            } else {
                Err(stream_closed())
            }
        },
    }
}

pub(crate) async fn await_io<T>(
    operation: impl Future<Output = std::io::Result<T>>,
    cancellation: Option<&CancellationToken>,
    deadlines: OperationDeadlines,
    io_message: &'static str,
) -> Result<T, HostError> {
    check_expired_deadlines(&deadlines)?;
    tokio::select! {
        biased;
        _ = wait_for_cancellation(cancellation) => Err(stream_cancelled()),
        _ = wait_until(deadlines.budget) => Err(time_budget_exceeded()),
        _ = wait_until(deadlines.operation) => Err(stream_timeout(deadlines.operation_timeout_ms)),
        result = operation => result.map_err(|error| stream_io_error(io_message, error)),
    }
}

fn check_expired_deadlines(deadlines: &OperationDeadlines) -> Result<(), HostError> {
    if let Some(error) = &deadlines.budget_error {
        return Err(error.clone());
    }
    if !deadlines.operation_deadline_valid {
        return Err(HostError::new(
            HostErrorCode::InvalidRequest,
            "stream operation timeout exceeds the runtime clock range",
        ));
    }
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
        HostErrorCode::Cancelled,
        "byte stream operation was cancelled",
    )
}

pub(crate) fn stream_closed() -> HostError {
    HostError::new(HostErrorCode::Closed, "byte stream is closed")
}

fn time_budget_exceeded() -> HostError {
    HostError::new(
        HostErrorCode::BudgetExceeded,
        "host operation exceeded its time budget",
    )
}

fn stream_timeout(timeout_ms: Option<u64>) -> HostError {
    HostError::new(HostErrorCode::TimedOut, "byte stream operation timed out")
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

    use super::{ByteStreamStore, ManagedStream, OperationDeadlines, StreamLifecycle};
    use crate::stream::local::read_until_limit;
    use crate::tls::local::tls_client_config;
    use crate::{
        AuthorityContext, Budget, ByteStreamOrigin, ByteStreamRef, CommandPolicy,
        DestructiveOpPolicy, ExecutionBudget, FilesystemPolicy, HostErrorCode, HostRequestId,
        LocalStreamClient, LocalTcpClient, LocalTlsClient, NetworkEndpoint, NetworkPolicy,
        SandboxPolicy, StreamClient, StreamOperation, StreamRequest, TcpClient,
        TcpConnectOperation, TcpConnectRequest, TcpEndpoint, TcpStreamRef, TimeBudget, TlsClient,
        TlsConnectOperation, TlsConnectRequest, TraceContext, TraceId,
    };
    use crate::{StreamFailure, StreamPayload, StreamRead};

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
                budget: ExecutionBudget::default(),
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
                budget: ExecutionBudget::default(),
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
    async fn tls_uses_store_provenance_instead_of_request_origin() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept client");
            stream
                .write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n")
                .await
                .expect("write plain response");
        });
        let streams = ByteStreamStore::new();
        let authority = allow_network("127.0.0.1", address.port());
        let tcp = LocalTcpClient::new(streams.clone())
            .execute(TcpConnectRequest {
                id: HostRequestId(50),
                operation: TcpConnectOperation::Connect {
                    endpoint: TcpEndpoint {
                        host: "127.0.0.1".to_owned(),
                        port: address.port(),
                    },
                },
                authority: authority.clone(),
                trace: TraceContext::root(TraceId(50)),
                budget: ExecutionBudget::default(),
            })
            .await
            .expect("TCP request should execute")
            .result
            .expect("TCP connect should succeed");
        let forged_reference = TcpStreamRef::issued(
            tcp.handle().clone(),
            ByteStreamOrigin::Tcp {
                host: "203.0.113.1".to_owned(),
                port: 9,
            },
        );

        let response = LocalTlsClient::new(streams)
            .execute(TlsConnectRequest {
                id: HostRequestId(51),
                operation: TlsConnectOperation::Connect {
                    stream: forged_reference,
                    server_name: "127.0.0.1".to_owned(),
                },
                authority,
                trace: TraceContext::root(TraceId(51)),
                budget: ExecutionBudget::default(),
            })
            .await
            .expect("TLS request should return a typed response");

        server.await.expect("server should finish");
        assert_eq!(
            response
                .result
                .expect_err("plain TCP must not complete a TLS handshake")
                .code,
            HostErrorCode::ProviderUnavailable,
            "request-provided origin must not influence authority checks"
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn denied_tls_authority_leaves_tcp_stream_open() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept client");
            let mut body = [0_u8; 2];
            stream
                .read_exact(&mut body)
                .await
                .expect("TCP stream should remain writable after denied TLS");
            body
        });
        let streams = ByteStreamStore::new();
        let tcp = LocalTcpClient::new(streams.clone())
            .execute(TcpConnectRequest {
                id: HostRequestId(52),
                operation: TcpConnectOperation::Connect {
                    endpoint: TcpEndpoint {
                        host: "127.0.0.1".to_owned(),
                        port: address.port(),
                    },
                },
                authority: allow_network("127.0.0.1", address.port()),
                trace: TraceContext::root(TraceId(52)),
                budget: ExecutionBudget::default(),
            })
            .await
            .expect("TCP request should execute")
            .result
            .expect("TCP connect should succeed");

        let denied = LocalTlsClient::new(streams.clone())
            .execute(TlsConnectRequest {
                id: HostRequestId(53),
                operation: TlsConnectOperation::Connect {
                    stream: tcp.clone(),
                    server_name: "127.0.0.1".to_owned(),
                },
                authority: AuthorityContext::deny_all(),
                trace: TraceContext::root(TraceId(53)),
                budget: ExecutionBudget::default(),
            })
            .await
            .expect("TLS request should return a typed response")
            .result
            .expect_err("TLS authority should be denied");
        assert_eq!(denied.code, HostErrorCode::AuthorityDenied);

        let write = LocalStreamClient::new(streams)
            .execute(stream_request(
                54,
                StreamOperation::WriteAll {
                    stream: tcp.as_byte_stream(),
                    body: b"ok".to_vec(),
                },
            ))
            .await
            .expect("stream write should execute");
        assert_eq!(write.result, Ok(StreamPayload::Unit));
        assert_eq!(server.await.expect("server task should finish"), *b"ok");
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
                budget: ExecutionBudget::default(),
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
            OperationDeadlines::new(&ExecutionBudget::default(), Some(2_000)),
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
            OperationDeadlines::new(&ExecutionBudget::default(), Some(2_000)),
        )
        .await
        .expect_err("limit must fail");
        assert_eq!(error, StreamFailure::LimitExceeded { limit_bytes: 2 });
        let mut restored = [0_u8; 3];
        let read = stream
            .read(&mut restored)
            .await
            .expect("overflow probe bytes should remain buffered");
        assert_eq!(read, 3);
        assert_eq!(&restored, b"abc");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn zero_length_read_returns_empty_data_without_consuming_eof() {
        let streams = ByteStreamStore::new();
        let stream_client = LocalStreamClient::new(streams.clone());
        let (stream, mut peer) = connected_stream().await;
        peer.write_all(b"x").await.expect("write one byte");
        let handle = streams.insert_stream_for_test("zero", stream).await;

        let empty = stream_client
            .execute(stream_request(
                8,
                StreamOperation::Read {
                    stream: ByteStreamRef::issued(handle.clone(), crate::ByteStreamOrigin::Opaque),
                    max_bytes: 0,
                    timeout_ms: None,
                },
            ))
            .await
            .expect("zero-length read should return a typed response");
        assert_eq!(
            empty.result,
            Ok(StreamPayload::Read(StreamRead::Data(Vec::new())))
        );

        let next = stream_client
            .execute(stream_request(
                9,
                StreamOperation::Read {
                    stream: ByteStreamRef::issued(handle, crate::ByteStreamOrigin::Opaque),
                    max_bytes: 1,
                    timeout_ms: Some(1_000),
                },
            ))
            .await
            .expect("subsequent read should return a typed response");
        assert_eq!(
            next.result,
            Ok(StreamPayload::Read(StreamRead::Data(vec![b'x'])))
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocked_read_does_not_block_other_stream_and_close_interrupts_it() {
        let streams = ByteStreamStore::new();
        let stream_client = LocalStreamClient::new(streams.clone());
        let (blocked_stream, _blocked_peer) = connected_stream().await;
        let (writable_stream, mut writable_peer) = connected_stream().await;
        let blocked_handle = streams
            .insert_stream_for_test("blocked", blocked_stream)
            .await;
        let writable_handle = streams
            .insert_stream_for_test("writable", writable_stream)
            .await;

        let read_client = stream_client.clone();
        let reader_handle = blocked_handle.clone();
        let reader = tokio::spawn(async move {
            read_client
                .execute(stream_request(
                    10,
                    StreamOperation::Read {
                        stream: ByteStreamRef::issued(
                            reader_handle,
                            crate::ByteStreamOrigin::Opaque,
                        ),
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
                    stream: ByteStreamRef::issued(writable_handle, crate::ByteStreamOrigin::Opaque),
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
                    stream: ByteStreamRef::issued(
                        blocked_handle.clone(),
                        crate::ByteStreamOrigin::Opaque,
                    ),
                },
            )),
        )
        .await
        .expect("close must not wait for the blocked read")
        .expect("close should cancel the read and complete the state transition");
        assert_eq!(close.result, Ok(StreamPayload::Unit));
        let read = timeout(Duration::from_secs(1), reader)
            .await
            .expect("close should cancel the blocked read")
            .expect("read task should not panic")
            .expect("blocked read host execution should complete");
        let error = read
            .result
            .expect_err("cancelled read must fail explicitly");
        assert_eq!(error, StreamFailure::Cancelled);

        let after_close = stream_client
            .execute(stream_request(
                13,
                StreamOperation::Read {
                    stream: ByteStreamRef::issued(
                        blocked_handle.clone(),
                        crate::ByteStreamOrigin::Opaque,
                    ),
                    max_bytes: 1,
                    timeout_ms: None,
                },
            ))
            .await
            .expect("closed stream request should return a typed response");
        assert_eq!(
            after_close
                .result
                .expect_err("closed stream must reject reads"),
            StreamFailure::Closed
        );

        let repeated_close = stream_client
            .execute(stream_request(
                14,
                StreamOperation::Close {
                    stream: ByteStreamRef::issued(blocked_handle, crate::ByteStreamOrigin::Opaque),
                },
            ))
            .await
            .expect("repeated close should return a typed response");
        assert_eq!(
            repeated_close
                .result
                .expect_err("repeated close must report closed state"),
            StreamFailure::Closed
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stream_operation_timeout_has_typed_error_code() {
        let streams = ByteStreamStore::new();
        let stream_client = LocalStreamClient::new(streams.clone());
        let (blocked_stream, _peer) = connected_stream().await;
        let handle = streams
            .insert_stream_for_test("timeout", blocked_stream)
            .await;

        let response = stream_client
            .execute(stream_request(
                20,
                StreamOperation::Read {
                    stream: ByteStreamRef::issued(handle, crate::ByteStreamOrigin::Opaque),
                    max_bytes: 1,
                    timeout_ms: Some(10),
                },
            ))
            .await
            .expect("timed out read should return a typed response");
        assert_eq!(
            response.result.expect_err("read must time out"),
            StreamFailure::TimedOut
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn unknown_stream_is_distinct_from_closed_stream() {
        let response = LocalStreamClient::new(ByteStreamStore::new())
            .execute(stream_request(
                30,
                StreamOperation::Read {
                    stream: ByteStreamRef::opaque_for_testing("never-created", 0),
                    max_bytes: 1,
                    timeout_ms: None,
                },
            ))
            .await
            .expect("unknown stream should return a typed response");
        let StreamFailure::Host(error) = response.result.expect_err("unknown stream must fail")
        else {
            panic!("unknown stream must remain a host protocol error");
        };
        assert_eq!(error.code, HostErrorCode::InvalidRequest);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn stale_stream_generation_is_rejected() {
        let streams = ByteStreamStore::new();
        let stream_client = LocalStreamClient::new(streams.clone());
        let (stream, _peer) = connected_stream().await;
        streams.insert_stream_for_test("generation", stream).await;

        let response = stream_client
            .execute(stream_request(
                31,
                StreamOperation::Read {
                    stream: ByteStreamRef::issued(
                        crate::StreamHandleRef::issued("generation", 1),
                        crate::ByteStreamOrigin::Opaque,
                    ),
                    max_bytes: 1,
                    timeout_ms: None,
                },
            ))
            .await
            .expect("stale stream should return a typed response");
        assert_eq!(
            response
                .result
                .expect_err("stale generation must not access the stream"),
            StreamFailure::Closed
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn locally_issued_stream_tokens_are_random_and_redacted() {
        let streams = ByteStreamStore::new();
        let (first, _first_peer) = connected_stream().await;
        let (second, _second_peer) = connected_stream().await;
        let first = streams
            .insert_stream(first, crate::ByteStreamOrigin::Opaque)
            .await
            .expect("issue first stream capability");
        let second = streams
            .insert_stream(second, crate::ByteStreamOrigin::Opaque)
            .await
            .expect("issue second stream capability");

        assert_ne!(first.token(), second.token());
        assert_eq!(first.token().len(), 32);
        assert!(!format!("{first:?}").contains(first.token()));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn close_during_tls_upgrade_cancels_handshake_without_stream_resurrection() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.expect("accept client");
            tokio::time::sleep(Duration::from_secs(2)).await;
        });
        let streams = ByteStreamStore::new();
        let tcp_client = LocalTcpClient::new(streams.clone());
        let tls_client = LocalTlsClient::new(streams.clone());
        let stream_client = LocalStreamClient::new(streams.clone());
        let authority = allow_network("127.0.0.1", address.port());
        let tcp = tcp_client
            .execute(TcpConnectRequest {
                id: HostRequestId(32),
                operation: TcpConnectOperation::Connect {
                    endpoint: TcpEndpoint {
                        host: "127.0.0.1".to_owned(),
                        port: address.port(),
                    },
                },
                authority: authority.clone(),
                trace: TraceContext::root(TraceId(32)),
                budget: ExecutionBudget::default(),
            })
            .await
            .expect("TCP request should execute")
            .result
            .expect("TCP connect should succeed");
        let slot = streams
            .stream_slot(tcp.handle())
            .await
            .expect("connected stream slot");
        let tls_tcp = tcp.clone();
        let tls = tokio::spawn(async move {
            tls_client
                .execute(TlsConnectRequest {
                    id: HostRequestId(33),
                    operation: TlsConnectOperation::Connect {
                        stream: tls_tcp,
                        server_name: "127.0.0.1".to_owned(),
                    },
                    authority,
                    trace: TraceContext::root(TraceId(33)),
                    budget: ExecutionBudget::default(),
                })
                .await
        });
        timeout(Duration::from_secs(1), async {
            while slot.lifecycle() != StreamLifecycle::Upgrading {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("TLS request should enter upgrading state");

        let close = stream_client
            .execute(stream_request(
                34,
                StreamOperation::Close {
                    stream: tcp.as_byte_stream(),
                },
            ))
            .await
            .expect("close request should execute");
        assert_eq!(close.result, Ok(StreamPayload::Unit));
        let tls = timeout(Duration::from_secs(1), tls)
            .await
            .expect("close should cancel the TLS handshake")
            .expect("TLS task should not panic")
            .expect("TLS host request should return a typed response");
        assert_eq!(
            tls.result
                .expect_err("cancelled upgrade must not create a TLS stream")
                .code,
            HostErrorCode::Cancelled
        );
        assert_eq!(slot.lifecycle(), StreamLifecycle::Closed);
        server.abort();
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
                budget: ExecutionBudget::start(Budget {
                    time: Some(TimeBudget { max_millis: 0 }),
                    ..Budget::default()
                }),
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
        let budget = ExecutionBudget::start(Budget {
            time: Some(TimeBudget { max_millis: 500 }),
            ..Budget::default()
        });
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
                budget: budget.clone(),
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
                budget: budget.with_limits(Budget {
                    time: Some(TimeBudget { max_millis: 25 }),
                    ..Budget::default()
                }),
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
        (ManagedStream::tcp(client), server)
    }

    fn stream_request(id: u32, operation: StreamOperation) -> StreamRequest {
        StreamRequest {
            id: HostRequestId(id),
            operation,
            authority: AuthorityContext::deny_all(),
            trace: TraceContext::root(TraceId(id)),
            budget: ExecutionBudget::default(),
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
