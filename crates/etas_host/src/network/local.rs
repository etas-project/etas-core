use std::{future::Future, pin::Pin};

use tokio::net::TcpStream;

use crate::{
    ByteStreamOrigin, HostError, HostErrorCode, SandboxBroker, TcpClient, TcpConnectOperation,
    TcpConnectRequest, TcpConnectResponse, TcpStreamRef,
    stream::{
        ByteStreamStore,
        store::{ManagedStream, OperationDeadlines, await_io, stream_io_error},
    },
};

#[derive(Clone, Default)]
pub struct LocalTcpClient {
    streams: ByteStreamStore,
}

impl LocalTcpClient {
    pub fn new(streams: ByteStreamStore) -> Self {
        Self { streams }
    }

    async fn execute_request(
        &self,
        request: TcpConnectRequest,
    ) -> Result<TcpConnectResponse, HostError> {
        match &request.operation {
            TcpConnectOperation::Connect { endpoint } => {
                let broker = SandboxBroker::new(request.authority.sandbox.clone());
                if let Err(error) =
                    broker.check_network_endpoint("tcp", &endpoint.host, endpoint.port)
                {
                    return Ok(TcpConnectResponse {
                        id: request.id,
                        result: Err(error),
                    });
                }
                let deadlines = OperationDeadlines::new(&request.budget, None);
                let resolved = await_io(
                    tokio::net::lookup_host((endpoint.host.as_str(), endpoint.port)),
                    None,
                    deadlines.clone(),
                    "TCP endpoint resolution failed",
                )
                .await;
                let addresses = match resolved.and_then(|addresses| {
                    broker.validate_resolved_network_addresses(
                        "tcp",
                        &endpoint.host,
                        endpoint.port,
                        addresses,
                    )
                }) {
                    Ok(addresses) => addresses,
                    Err(error) => {
                        return Ok(TcpConnectResponse {
                            id: request.id,
                            result: Err(error),
                        });
                    }
                };
                let mut last_error: Option<HostError> = None;
                let mut stream = None;
                for address in &addresses {
                    match await_io(
                        TcpStream::connect(address),
                        None,
                        deadlines.clone(),
                        "TCP connect failed",
                    )
                    .await
                    {
                        Ok(connected) => {
                            stream = Some(connected);
                            break;
                        }
                        Err(error) if error.code == HostErrorCode::BudgetExceeded => {
                            return Ok(TcpConnectResponse {
                                id: request.id,
                                result: Err(error.with_detail("address", address.to_string())),
                            });
                        }
                        Err(error) => {
                            last_error = Some(error.with_detail("address", address.to_string()))
                        }
                    }
                }
                let Some(stream) = stream else {
                    return Ok(TcpConnectResponse {
                        id: request.id,
                        result: Err(last_error.unwrap_or_else(|| {
                            HostError::new(
                                HostErrorCode::ProviderUnavailable,
                                "TCP endpoint resolved to no usable addresses",
                            )
                        })),
                    });
                };
                if let Err(error) = stream.set_nodelay(true) {
                    return Ok(TcpConnectResponse {
                        id: request.id,
                        result: Err(stream_io_error("failed to configure TCP stream", error)),
                    });
                }
                let origin = ByteStreamOrigin::Tcp {
                    host: endpoint.host.clone(),
                    port: endpoint.port,
                };
                let handle = match self
                    .streams
                    .insert_stream(ManagedStream::tcp(stream), origin.clone())
                    .await
                {
                    Ok(handle) => handle,
                    Err(error) => {
                        return Ok(TcpConnectResponse {
                            id: request.id,
                            result: Err(error),
                        });
                    }
                };
                Ok(TcpConnectResponse {
                    id: request.id,
                    result: Ok(TcpStreamRef::issued(handle, origin)),
                })
            }
        }
    }
}

impl TcpClient for LocalTcpClient {
    type Error = HostError;
    type ExecuteFuture<'a> =
        Pin<Box<dyn Future<Output = Result<TcpConnectResponse, Self::Error>> + Send + 'a>>;

    fn execute(&self, request: TcpConnectRequest) -> Self::ExecuteFuture<'_> {
        Box::pin(self.execute_request(request))
    }
}

#[cfg(test)]
mod tests {
    use crate::{HostError, TcpClient};

    use super::LocalTcpClient;

    fn assert_tcp_client<T: TcpClient<Error = HostError>>() {}

    #[test]
    fn local_tcp_client_implements_service_trait() {
        assert_tcp_client::<LocalTcpClient>();
    }
}
