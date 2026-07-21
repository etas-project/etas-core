use std::{
    any::Any,
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    net::TcpStream as StdTcpStream,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use rustls_pki_types::ServerName;

use crate::{
    ByteStreamOrigin, HostError, HostErrorCode, SandboxBroker, StreamOperation, StreamPayload,
    StreamRead, StreamRequest, StreamResponse, TcpConnectOperation, TcpConnectRequest,
    TcpConnectResponse, TcpStreamRef, TlsConnectOperation, TlsConnectRequest, TlsConnectResponse,
    TlsStreamRef,
};

#[derive(Clone, Default)]
pub struct LocalTcpStreamClient {
    next_stream_id: Arc<AtomicU64>,
    streams: Arc<Mutex<HashMap<String, ManagedStream>>>,
}

enum ManagedStream {
    Tcp(StdTcpStream),
    Tls(StreamOwned<ClientConnection, StdTcpStream>),
}

impl ManagedStream {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> std::io::Result<()> {
        match self {
            ManagedStream::Tcp(stream) => stream.set_read_timeout(timeout),
            ManagedStream::Tls(stream) => stream.sock.set_read_timeout(timeout),
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ManagedStream::Tcp(stream) => stream.read(buffer),
            ManagedStream::Tls(stream) => stream.read(buffer),
        }
    }

    fn write_all(&mut self, body: &[u8]) -> std::io::Result<()> {
        match self {
            ManagedStream::Tcp(stream) => stream.write_all(body),
            ManagedStream::Tls(stream) => stream.write_all(body),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            ManagedStream::Tcp(stream) => stream.flush(),
            ManagedStream::Tls(stream) => stream.flush(),
        }
    }
}

impl LocalTcpStreamClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn execute_tcp(
        &self,
        request: TcpConnectRequest,
    ) -> Result<TcpConnectResponse, HostError> {
        match &request.operation {
            TcpConnectOperation::Connect { endpoint } => {
                let addresses = match SandboxBroker::new(request.authority.sandbox.clone())
                    .resolve_network_endpoint("tcp", &endpoint.host, endpoint.port)
                {
                    Ok(addresses) => addresses,
                    Err(error) => {
                        return Ok(TcpConnectResponse {
                            id: request.id,
                            result: Err(error),
                        });
                    }
                };
                let mut last_error = None;
                let mut stream = None;
                for address in &addresses {
                    match StdTcpStream::connect(address) {
                        Ok(connected) => {
                            stream = Some(connected);
                            break;
                        }
                        Err(error) => last_error = Some((address, error)),
                    }
                }
                let Some(stream) = stream else {
                    let (address, error) = last_error
                        .expect("resolved TCP endpoint must contain at least one address");
                    return Ok(TcpConnectResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::ProviderUnavailable,
                            "TCP connect failed",
                        )
                        .with_detail("address", address.to_string())
                        .with_detail("error", error.to_string())),
                    });
                };
                let _ = stream.set_nodelay(true);
                let id = format!(
                    "tcp:{}:{}:{}",
                    endpoint.host,
                    endpoint.port,
                    self.next_stream_id.fetch_add(1, Ordering::Relaxed)
                );
                self.streams
                    .lock()
                    .expect("TCP stream table lock")
                    .insert(id.clone(), ManagedStream::Tcp(stream));
                Ok(TcpConnectResponse {
                    id: request.id,
                    result: Ok(TcpStreamRef {
                        id,
                        origin: ByteStreamOrigin::Tcp {
                            host: endpoint.host.clone(),
                            port: endpoint.port,
                        },
                    }),
                })
            }
        }
    }

    pub async fn execute_stream(
        &self,
        request: StreamRequest,
    ) -> Result<StreamResponse, HostError> {
        let mut streams = self.streams.lock().expect("TCP stream table lock");
        let response = match &request.operation {
            StreamOperation::Read {
                stream,
                max_bytes,
                timeout_ms,
            } => {
                let Some(connection) = streams.get_mut(&stream.id) else {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(unknown_stream(&stream.id)),
                    });
                };
                if let Err(error) =
                    connection.set_read_timeout(timeout_ms.map(Duration::from_millis))
                {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(stream_io_error(
                            "failed to configure stream read timeout",
                            error,
                        )),
                    });
                }
                let mut buffer = vec![0; *max_bytes];
                match connection.read(&mut buffer) {
                    Ok(0) => Ok(StreamPayload::Read(StreamRead::Eof)),
                    Ok(read) => {
                        buffer.truncate(read);
                        Ok(StreamPayload::Read(StreamRead::Data(buffer)))
                    }
                    Err(error) => Err(stream_io_error("stream read failed", error)),
                }
            }
            StreamOperation::ReadUntilLimit {
                stream,
                limit_bytes,
                timeout_ms,
            } => {
                let Some(connection) = streams.get_mut(&stream.id) else {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(unknown_stream(&stream.id)),
                    });
                };
                read_until_limit(connection, *limit_bytes, *timeout_ms)
            }
            StreamOperation::WriteAll { stream, body } => {
                let Some(connection) = streams.get_mut(&stream.id) else {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(unknown_stream(&stream.id)),
                    });
                };
                match connection.write_all(body).and_then(|()| connection.flush()) {
                    Ok(()) => Ok(StreamPayload::Unit),
                    Err(error) => Err(stream_io_error("stream write failed", error)),
                }
            }
            StreamOperation::Flush { stream } => {
                let Some(connection) = streams.get_mut(&stream.id) else {
                    return Ok(StreamResponse {
                        id: request.id,
                        result: Err(unknown_stream(&stream.id)),
                    });
                };
                match connection.flush() {
                    Ok(()) => Ok(StreamPayload::Unit),
                    Err(error) => Err(stream_io_error("stream flush failed", error)),
                }
            }
            StreamOperation::Close { stream } => {
                streams.remove(&stream.id);
                Ok(StreamPayload::Unit)
            }
        };
        Ok(StreamResponse {
            id: request.id,
            result: response,
        })
    }

    pub async fn execute_tls(
        &self,
        request: TlsConnectRequest,
    ) -> Result<TlsConnectResponse, HostError> {
        match &request.operation {
            TlsConnectOperation::Connect {
                stream,
                server_name,
            } => {
                let ByteStreamOrigin::Tcp {
                    host: tcp_host,
                    port: tcp_port,
                } = &stream.origin
                else {
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::InvalidRequest,
                            "TLS handshake requires a TCP stream with typed TCP origin",
                        )
                        .with_detail("stream", stream.id.clone())),
                    });
                };
                if let Err(error) = SandboxBroker::new(request.authority.sandbox.clone())
                    .check_network_endpoint("tls", server_name, *tcp_port)
                {
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(error),
                    });
                }
                let tcp_stream = {
                    let mut streams = self.streams.lock().expect("TCP stream table lock");
                    let Some(managed) = streams.remove(&stream.id) else {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(unknown_stream(&stream.id)),
                        });
                    };
                    match managed {
                        ManagedStream::Tcp(stream) => stream,
                        ManagedStream::Tls(_) => {
                            return Ok(TlsConnectResponse {
                                id: request.id,
                                result: Err(HostError::new(
                                    HostErrorCode::InvalidRequest,
                                    "TLS handshake requires a plain TCP stream",
                                )
                                .with_detail("stream", stream.id.clone())),
                            });
                        }
                    }
                };
                let tls_stream = match tls_handshake(tcp_stream, server_name) {
                    Ok(stream) => stream,
                    Err(error) => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(error
                                .with_detail("host", tcp_host.clone())
                                .with_detail("port", tcp_port.to_string())
                                .with_detail("server_name", server_name.clone())),
                        });
                    }
                };
                let id = format!(
                    "tls:{}:{}:{}:{}",
                    tcp_host,
                    tcp_port,
                    server_name,
                    self.next_stream_id.fetch_add(1, Ordering::Relaxed)
                );
                self.streams
                    .lock()
                    .expect("TCP stream table lock")
                    .insert(id.clone(), ManagedStream::Tls(tls_stream));
                Ok(TlsConnectResponse {
                    id: request.id,
                    result: Ok(TlsStreamRef {
                        id,
                        origin: ByteStreamOrigin::Tls {
                            host: tcp_host.clone(),
                            port: *tcp_port,
                            server_name: Some(server_name.clone()),
                        },
                    }),
                })
            }
        }
    }
}

fn unknown_stream(id: &str) -> HostError {
    HostError::new(HostErrorCode::InvalidRequest, "unknown byte stream")
        .with_detail("stream", id.to_owned())
}

fn stream_io_error(message: &'static str, error: std::io::Error) -> HostError {
    HostError::new(HostErrorCode::ProviderUnavailable, message)
        .with_detail("error", error.to_string())
}

fn read_until_limit(
    connection: &mut ManagedStream,
    limit_bytes: usize,
    timeout_ms: Option<u64>,
) -> Result<StreamPayload, HostError> {
    let timeout = timeout_ms.map(Duration::from_millis);
    let started = Instant::now();
    let mut body = Vec::new();

    loop {
        let read_timeout = timeout.map(|timeout| {
            timeout
                .checked_sub(started.elapsed())
                .unwrap_or(Duration::ZERO)
        });
        if matches!(read_timeout, Some(timeout) if timeout.is_zero()) {
            return Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "stream read-until-limit timed out",
            )
            .with_detail("timeout_ms", timeout_ms.unwrap_or_default().to_string()));
        }
        if let Err(error) = connection.set_read_timeout(read_timeout) {
            return Err(stream_io_error(
                "failed to configure stream read timeout",
                error,
            ));
        }

        let remaining = limit_bytes.saturating_sub(body.len());
        let chunk_size = if remaining == 0 {
            1
        } else {
            remaining.min(8 * 1024)
        };
        let mut chunk = vec![0; chunk_size];
        match connection.read(&mut chunk) {
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
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Err(HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "stream read-until-limit timed out",
                )
                .with_detail("timeout_ms", timeout_ms.unwrap_or_default().to_string()));
            }
            Err(error) => return Err(stream_io_error("stream read-until-limit failed", error)),
        }
    }
}

fn tls_handshake(
    tcp_stream: StdTcpStream,
    server_name: &str,
) -> Result<StreamOwned<ClientConnection, StdTcpStream>, HostError> {
    match catch_unwind(AssertUnwindSafe(|| {
        tls_handshake_inner(tcp_stream, server_name)
    })) {
        Ok(result) => result,
        Err(payload) => Err(HostError::new(
            HostErrorCode::ProviderUnavailable,
            "TLS host boundary failed",
        )
        .with_detail("panic", panic_payload_message(payload.as_ref()))),
    }
}

fn tls_handshake_inner(
    tcp_stream: StdTcpStream,
    server_name: &str,
) -> Result<StreamOwned<ClientConnection, StdTcpStream>, HostError> {
    let config = tls_client_config()?;
    let server_name = ServerName::try_from(server_name.to_owned()).map_err(|error| {
        HostError::new(HostErrorCode::InvalidRequest, "invalid TLS server name")
            .with_detail("error", error.to_string())
    })?;
    let connection = ClientConnection::new(Arc::new(config), server_name).map_err(|error| {
        HostError::new(
            HostErrorCode::ProviderUnavailable,
            "TLS client setup failed",
        )
        .with_detail("error", error.to_string())
    })?;
    let mut tls_stream = StreamOwned::new(connection, tcp_stream);
    while tls_stream.conn.is_handshaking() {
        tls_stream
            .conn
            .complete_io(&mut tls_stream.sock)
            .map_err(|error| {
                HostError::new(HostErrorCode::ProviderUnavailable, "TLS handshake failed")
                    .with_detail("error", error.to_string())
            })?;
    }
    Ok(tls_stream)
}

fn tls_client_config() -> Result<ClientConfig, HostError> {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    Ok(ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth())
}

fn panic_payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, net::TcpListener, panic::AssertUnwindSafe, thread};

    use super::{LocalTcpStreamClient, ManagedStream, read_until_limit, tls_client_config};
    use crate::{
        AuthorityContext, Budget, CommandPolicy, DestructiveOpPolicy, FilesystemPolicy,
        HostErrorCode, HostRequestId, NetworkEndpoint, NetworkPolicy, SandboxPolicy,
        TcpConnectOperation, TcpConnectRequest, TcpEndpoint, TlsConnectOperation,
        TlsConnectRequest, TraceContext, TraceId,
    };
    use crate::{StreamPayload, StreamRead};

    #[test]
    fn tls_client_config_builder_does_not_panic() {
        let result = std::panic::catch_unwind(AssertUnwindSafe(tls_client_config));
        assert!(result.is_ok(), "TLS client config construction panicked");
        result
            .expect("panic checked above")
            .expect("TLS client config should build");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn tls_connect_to_plain_tcp_server_returns_host_error_without_panic() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept client");
            let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n");
        });
        let client = LocalTcpStreamClient::new();
        let authority = allow_network("127.0.0.1", address.port());

        let tcp = client
            .execute_tcp(TcpConnectRequest {
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

        let tls = client
            .execute_tls(TlsConnectRequest {
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

        server.join().expect("server should finish");
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
        let client = LocalTcpStreamClient::new();
        let response = client
            .execute_tcp(TcpConnectRequest {
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

    #[test]
    fn read_until_limit_accumulates_until_eof() {
        let (mut stream, mut writer) = connected_stream();
        let writer = thread::spawn(move || {
            writer.write_all(b"hello ").expect("write first chunk");
            writer.write_all(b"world").expect("write second chunk");
        });

        let payload = read_until_limit(&mut stream, 32, Some(2_000)).expect("read should succeed");
        writer.join().expect("writer should finish");
        assert_eq!(
            payload,
            StreamPayload::Read(StreamRead::Data(b"hello world".to_vec()))
        );
    }

    #[test]
    fn read_until_limit_fails_when_body_exceeds_limit() {
        let (mut stream, mut writer) = connected_stream();
        let writer = thread::spawn(move || {
            writer.write_all(b"abc").expect("write body");
        });

        let error = read_until_limit(&mut stream, 2, Some(2_000)).expect_err("limit must fail");
        writer.join().expect("writer should finish");
        assert_eq!(error.code, crate::HostErrorCode::BudgetExceeded);
        assert!(
            error.message.contains("exceeded byte limit"),
            "{}",
            error.message
        );
    }

    fn connected_stream() -> (ManagedStream, std::net::TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local listener");
        let address = listener.local_addr().expect("local addr");
        let client = std::net::TcpStream::connect(address).expect("connect client");
        let (server, _) = listener.accept().expect("accept client");
        (ManagedStream::Tcp(client), server)
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
