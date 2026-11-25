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
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<TlsConnectResponse, HostError> {
        if let Some(operation) = operation {
            operation.signal().check()?;
        }
        match &request.operation {
            TlsConnectOperation::Connect {
                stream,
                server_name,
                options,
            } => {
                let config = match tls_client_config(options) {
                    Ok(config) => config,
                    Err(error) => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(error),
                        });
                    }
                };
                let timeout = match options.timeout_ms.map(u64::try_from).transpose() {
                    Ok(timeout) => timeout,
                    Err(_) => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(HostError::new(
                                HostErrorCode::InvalidRequest,
                                "TLS timeout exceeds the runtime clock range",
                            )),
                        });
                    }
                };
                request.budget.check_time()?;
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
                let Some(slot) = self.streams.stream_slot(stream.handle()).await else {
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(unknown_stream(stream.handle())),
                    });
                };
                let origin = match slot.validate_tls_upgrade(stream.handle()) {
                    Ok(origin) => origin,
                    Err(error) => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(
                                error.with_detail("stream", stream.handle().identity_fingerprint())
                            ),
                        });
                    }
                };
                let (checked_tcp_host, checked_tcp_port) = match origin {
                    ByteStreamOrigin::Tcp { host, port } => (host, port),
                    _ => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(HostError::new(
                                HostErrorCode::InvalidRequest,
                                "TLS handshake requires a host-issued TCP stream",
                            )
                            .with_detail("stream", stream.handle().identity_fingerprint())),
                        });
                    }
                };
                if let Err(error) = SandboxBroker::new(request.authority.sandbox.clone())
                    .check_network_endpoint("tls", server_name, checked_tcp_port)
                {
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(error),
                    });
                }
                if let Some(operation) = operation {
                    operation.signal().check()?;
                }
                let (tcp_stream, origin, _upgrade) = match slot.begin_tls_upgrade(stream.handle()) {
                    Ok(upgrade) => upgrade,
                    Err(error) => {
                        return Ok(TlsConnectResponse {
                            id: request.id,
                            result: Err(
                                error.with_detail("stream", stream.handle().identity_fingerprint())
                            ),
                        });
                    }
                };
                let ByteStreamOrigin::Tcp {
                    host: tcp_host,
                    port: tcp_port,
                } = origin
                else {
                    slot.fail_tls_upgrade().await;
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::InvalidResponse,
                            "TCP stream provenance changed during TLS upgrade",
                        )
                        .with_detail("expected_host", checked_tcp_host)),
                    });
                };
                if tcp_host != checked_tcp_host || tcp_port != checked_tcp_port {
                    slot.fail_tls_upgrade().await;
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::InvalidResponse,
                            "TCP stream provenance changed during TLS upgrade",
                        )),
                    });
                }
                let deadlines =
                    OperationDeadlines::new(&request.budget, timeout).with_operation(operation);
                let tls_stream = match tls_handshake(
                    tcp_stream,
                    server_name_value,
                    &slot.cancellation,
                    deadlines,
                    config,
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
                if !options.alpn.is_empty() && tls_stream.get_ref().1.alpn_protocol().is_none() {
                    slot.fail_tls_upgrade().await;
                    return Ok(TlsConnectResponse {
                        id: request.id,
                        result: Err(HostError::new(
                            HostErrorCode::ProviderRejected,
                            "TLS peer did not negotiate a requested application protocol",
                        )
                        .with_detail("tls_error", "protocol")),
                    });
                }
                let tls_origin = ByteStreamOrigin::Tls {
                    host: tcp_host.clone(),
                    port: tcp_port,
                    server_name: Some(server_name.clone()),
                };
                let generation = match slot
                    .finish_tls_upgrade(tls_stream, tls_origin.clone())
                    .await
                {
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
                        tls_origin,
                    )),
                })
            }
        }
    }

    pub async fn execute_scoped(
        &self,
        request: TlsConnectRequest,
        operation: &crate::execution::OperationContext,
    ) -> Result<TlsConnectResponse, HostError> {
        let client = self.clone();
        operation
            .supervise(move |context| async move {
                client.execute_request(request, Some(&context)).await
            })
            .await
    }
}

impl TlsClient for LocalTlsClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<TlsConnectResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: TlsConnectRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.execute_request(request, None))
    }
}

async fn tls_handshake(
    tcp_stream: TcpStream,
    server_name: ServerName<'static>,
    cancellation: &tokio_util::sync::CancellationToken,
    deadlines: OperationDeadlines,
    config: ClientConfig,
) -> Result<TlsStream<TcpStream>, HostError> {
    let connector = TlsConnector::from(Arc::new(config));
    await_io(
        connector.connect(server_name, tcp_stream),
        Some(cancellation),
        deadlines,
        "TLS handshake failed",
    )
    .await
}

pub(crate) fn tls_client_config(
    options: &crate::TlsConnectOptions,
) -> Result<ClientConfig, HostError> {
    use rustls_pki_types::{CertificateDer, pem::PemObject};
    let invalid = || HostError::new(HostErrorCode::InvalidRequest, "invalid TLS configuration");
    let mut roots = RootCertStore::empty();
    match &options.trust {
        crate::TlsTrust::Platform => roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
        crate::TlsTrust::Bundle(bytes) => {
            let mut certificates = CertificateDer::pem_slice_iter(bytes)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| invalid())?;
            if certificates.is_empty() {
                certificates.push(CertificateDer::from(bytes.clone()));
            }
            for certificate in certificates {
                roots.add(certificate).map_err(|_| invalid())?;
            }
        }
    }
    let versions: &[&rustls::SupportedProtocolVersion] = match options.minimum {
        crate::TlsVersion::V12 => &[&rustls::version::TLS13, &rustls::version::TLS12],
        crate::TlsVersion::V13 => &[&rustls::version::TLS13],
    };
    let mut config = ClientConfig::builder_with_protocol_versions(versions)
        .with_root_certificates(roots)
        .with_no_client_auth();
    for protocol in &options.alpn {
        if protocol.is_empty() || protocol.len() > 255 {
            return Err(invalid());
        }
        config.alpn_protocols.push(protocol.as_bytes().to_vec());
    }
    Ok(config)
}
