use std::{future::Future, pin::Pin, sync::Arc};

use rustls::{ClientConfig, RootCertStore};
use rustls_pki_types::ServerName;
use tokio::net::TcpStream;
use tokio_rustls::{TlsConnector, client::TlsStream};

use crate::{
    ByteStreamOrigin, HostError, HostErrorCode, SandboxBroker, TlsClient, TlsConnectOperation,
    TlsConnectRequest, TlsConnectResponse, TlsStreamRef,
    stream::{
        ByteStreamStore,
        store::{OperationDeadlines, await_io, unknown_stream},
    },
};

#[derive(Clone, Default)]
pub struct LocalTlsClient {
    streams: ByteStreamStore,
}

impl LocalTlsClient {
    pub fn new(streams: ByteStreamStore) -> Self {
        Self { streams }
    }

    async fn execute_request(
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
                } = stream.origin()
                else {
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::InvalidRequest,
                            "TLS handshake requires a TCP stream with typed TCP origin",
                        )
                        .with_detail("stream", stream.handle().identity_fingerprint())),
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
                let server_name_value = match ServerName::try_from(server_name.clone()) {
                    Ok(server_name) => server_name,
                    Err(error) => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(HostError::new(
                                HostErrorCode::InvalidRequest,
                                "invalid TLS server name",
                            )
                            .with_detail("error", error.to_string())),
                        });
                    }
                };
                let tcp_stream = {
                    let Some(slot) = self.streams.stream_slot(stream.handle()).await else {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(unknown_stream(stream.handle())),
                        });
                    };
                    match slot.begin_tls_upgrade(stream.handle()) {
                        Ok(tcp_stream) => (slot, tcp_stream),
                        Err(error) => {
                            return Ok(TlsConnectResponse {
                                id: request.id,
                                result: Err(error
                                    .with_detail("stream", stream.handle().identity_fingerprint())),
                            });
                        }
                    }
                };
                let deadlines = OperationDeadlines::new(&request.budget, None);
                let (slot, tcp_stream) = tcp_stream;
                let tls_stream = match tls_handshake(
                    tcp_stream,
                    server_name_value,
                    &slot.cancellation,
                    deadlines,
                )
                .await
                {
                    Ok(stream) => stream,
                    Err(error) => {
                        slot.fail_tls_upgrade().await;
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(error
                                .with_detail("host", tcp_host.clone())
                                .with_detail("port", tcp_port.to_string())
                                .with_detail("server_name", server_name.clone())),
                        });
                    }
                };
                let generation = match slot.finish_tls_upgrade(tls_stream).await {
                    Ok(generation) => generation,
                    Err(error) => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(error),
                        });
                    }
                };
                Ok(TlsConnectResponse {
                    id: request.id,
                    result: Ok(TlsStreamRef::issued(
                        crate::StreamHandleRef::issued(
                            stream.handle().token().to_owned(),
                            generation,
                        ),
                        ByteStreamOrigin::Tls {
                            host: tcp_host.clone(),
                            port: *tcp_port,
                            server_name: Some(server_name.clone()),
                        },
                    )),
                })
            }
        }
    }
}

impl TlsClient for LocalTlsClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<TlsConnectResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: TlsConnectRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.execute_request(request))
    }
}

async fn tls_handshake(
    tcp_stream: TcpStream,
    server_name: ServerName<'static>,
    cancellation: &tokio_util::sync::CancellationToken,
    deadlines: OperationDeadlines,
) -> Result<TlsStream<TcpStream>, HostError> {
    let config = tls_client_config();
    let connector = TlsConnector::from(Arc::new(config));
    await_io(
        connector.connect(server_name, tcp_stream),
        Some(cancellation),
        deadlines,
        "TLS handshake failed",
    )
    .await
}

pub(crate) fn tls_client_config() -> ClientConfig {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth()
}
