use std::{net::SocketAddr, time::Duration};

use reqwest::{
    Client, Method,
    header::{HeaderName, HeaderValue},
};

use crate::{
    AuthConfig, HostError, HostErrorCode, PrivateResolutionPolicy, RetryPolicy, TimeoutConfig,
};

use super::{TransportEndpointAuthority, TransportEndpointResolver};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpTransport {
    authority: TransportEndpointAuthority,
    pub auth: AuthConfig,
    pub timeout: TimeoutConfig,
    pub retry: RetryPolicy,
}

impl HttpTransport {
    pub fn try_new(
        base_url: impl AsRef<str>,
        private_resolution: PrivateResolutionPolicy,
    ) -> Result<Self, HostError> {
        Ok(Self {
            authority: TransportEndpointAuthority::try_new(base_url, private_resolution)?,
            auth: AuthConfig::None,
            timeout: TimeoutConfig::local(),
            retry: RetryPolicy::none(),
        })
    }

    pub fn base_url(&self) -> &str {
        self.authority.base_url()
    }

    pub fn with_auth(mut self, auth: AuthConfig) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_timeout(mut self, timeout: TimeoutConfig) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub async fn send_json(&self, path: &str, body: String) -> Result<HttpResponse, HostError> {
        if self.retry.attempts == 0 {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "HTTP retry policy must attempt each request at least once",
            ));
        }
        let mut last_error = None;
        for attempt in 0..self.retry.attempts {
            match self
                .send_once(HttpRequest {
                    method: "POST".to_owned(),
                    path: path.to_owned(),
                    headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                    body: body.clone(),
                })
                .await
            {
                Ok(response) => return Ok(response),
                Err(error) => {
                    last_error = Some(error);
                    if attempt + 1 < self.retry.attempts {
                        tokio::time::sleep(self.retry.delay).await;
                    }
                }
            }
        }
        match last_error {
            Some(error) => Err(error),
            None => Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "HTTP retry policy made no request attempts",
            )),
        }
    }

    pub async fn send_once(&self, request: HttpRequest) -> Result<HttpResponse, HostError> {
        let url = self.authority.join(&request.path)?;
        let client = resolved_http_client(&self.authority)?;
        let method = Method::from_bytes(request.method.as_bytes()).map_err(|error| {
            HostError::new(HostErrorCode::InvalidRequest, "invalid HTTP method")
                .with_detail("error", error.to_string())
        })?;
        let mut builder = client
            .request(method, url)
            .timeout(total_timeout(self.timeout))
            .body(request.body);

        for (name, value) in request.headers.into_iter().chain(self.auth.headers()) {
            builder = builder.header(
                parse_header_name(&name)?,
                parse_header_value(&name, &value)?,
            );
        }

        let response = builder.send().await.map_err(|error| {
            HostError::new(HostErrorCode::ProviderUnavailable, "HTTP request failed")
                .with_detail("error", error.to_string())
        })?;
        let status = response.status().as_u16();
        let headers = response_headers(response.headers())?;
        let bytes = response.bytes().await.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to read HTTP response",
            )
            .with_detail("error", error.to_string())
        })?;
        let body = String::from_utf8(bytes.to_vec()).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "HTTP response body is not valid UTF-8",
            )
            .with_detail("error", error.to_string())
        })?;
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }

    pub async fn send_raw(
        &self,
        method: impl AsRef<str>,
        url: impl AsRef<str>,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<HttpRawResponse, HostError> {
        let url = self.authority.parse_and_authorize(url.as_ref())?;
        let client = resolved_http_client(&self.authority)?;
        let method = Method::from_bytes(method.as_ref().as_bytes()).map_err(|error| {
            HostError::new(HostErrorCode::InvalidRequest, "invalid HTTP method")
                .with_detail("error", error.to_string())
        })?;
        let mut builder = client
            .request(method, url)
            .timeout(total_timeout(self.timeout))
            .body(body);
        for (name, value) in headers.into_iter().chain(self.auth.headers()) {
            builder = builder.header(
                parse_header_name(&name)?,
                parse_header_value(&name, &value)?,
            );
        }
        let response = builder.send().await.map_err(|error| {
            HostError::new(HostErrorCode::ProviderUnavailable, "HTTP request failed")
                .with_detail("error", error.to_string())
        })?;
        let status = response.status().as_u16();
        let headers = response_headers(response.headers())?;
        let body = response.bytes().await.map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to read HTTP response",
            )
            .with_detail("error", error.to_string())
        })?;
        Ok(HttpRawResponse {
            status,
            headers,
            body: body.to_vec(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRawResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

fn resolved_http_client(authority: &TransportEndpointAuthority) -> Result<Client, HostError> {
    let (_, host, _) = authority.endpoint();
    let addresses = TransportEndpointResolver::resolve(authority)?;
    build_resolved_http_client(host, &addresses)
}

fn build_resolved_http_client(host: &str, addresses: &[SocketAddr]) -> Result<Client, HostError> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .resolve_to_addrs(host, addresses)
        .build()
        .map_err(|error| {
            HostError::new(
                HostErrorCode::ProviderUnavailable,
                "failed to configure resolved HTTP endpoint",
            )
            .with_detail("host", host)
            .with_detail("error", error.to_string())
        })
}

fn total_timeout(timeout: TimeoutConfig) -> Duration {
    timeout.connect + timeout.read + timeout.write
}

fn response_headers(
    headers: &reqwest::header::HeaderMap,
) -> Result<Vec<(String, String)>, HostError> {
    headers
        .iter()
        .map(|(name, value)| {
            Ok((
                name.as_str().to_owned(),
                value
                    .to_str()
                    .map_err(|error| {
                        HostError::new(
                            HostErrorCode::InvalidResponse,
                            "HTTP response contains a non-UTF-8 header",
                        )
                        .with_detail("header", name.as_str())
                        .with_detail("error", error.to_string())
                    })?
                    .to_owned(),
            ))
        })
        .collect()
}

fn parse_header_name(name: &str) -> Result<HeaderName, HostError> {
    HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
        HostError::new(
            HostErrorCode::InvalidRequest,
            "invalid HTTP request header name",
        )
        .with_detail("header", name)
        .with_detail("error", error.to_string())
    })
}

fn parse_header_value(name: &str, value: &str) -> Result<HeaderValue, HostError> {
    HeaderValue::from_str(value).map_err(|error| {
        HostError::new(
            HostErrorCode::InvalidRequest,
            "invalid HTTP request header value",
        )
        .with_detail("header", name)
        .with_detail("error", error.to_string())
    })
}
