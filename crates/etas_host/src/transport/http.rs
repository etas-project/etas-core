use std::{cmp, net::SocketAddr, time::Duration};

use reqwest::{
    Client, Method,
    header::{HeaderName, HeaderValue},
};

use crate::{
    AuthConfig, HostError, HostErrorCode, PrivateResolutionPolicy, RetryPolicy,
    TransportTimeoutPolicy,
};

use super::{TransportEndpointAuthority, TransportEndpointResolver};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpTransport {
    authority: TransportEndpointAuthority,
    pub auth: AuthConfig,
    pub timeout: TransportTimeoutPolicy,
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
            timeout: TransportTimeoutPolicy::default(),
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

    pub fn with_timeout(mut self, timeout: TransportTimeoutPolicy) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub async fn send_json(&self, path: &str, body: String) -> Result<HttpResponse, HostError> {
        self.send_json_with_deadline(path, body, None).await
    }

    pub async fn send_json_with_deadline(
        &self,
        path: &str,
        body: String,
        outer_deadline: Option<tokio::time::Instant>,
    ) -> Result<HttpResponse, HostError> {
        self.send_json_inner(path, body, outer_deadline, None).await
    }

    pub async fn send_json_scoped(
        &self,
        path: &str,
        body: String,
        outer_deadline: Option<tokio::time::Instant>,
        operation: &crate::execution::OperationContext,
    ) -> Result<HttpResponse, HostError> {
        self.send_json_inner(path, body, outer_deadline, Some(operation))
            .await
    }

    async fn send_json_inner(
        &self,
        path: &str,
        body: String,
        outer_deadline: Option<tokio::time::Instant>,
        operation: Option<&crate::execution::OperationContext>,
    ) -> Result<HttpResponse, HostError> {
        if self.retry.attempts == 0 {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "HTTP retry policy must attempt each request at least once",
            ));
        }
        let deadline = self.effective_deadline(outer_deadline);
        let mut last_error = None;
        for attempt in 0..self.retry.attempts {
            let registration = operation
                .map(|operation| operation.register_work())
                .transpose()?;
            if let Some(registration) = &registration {
                registration.begin_dispatch()?;
            }
            let attempt_context = registration
                .as_ref()
                .map(|registration| registration.context());
            let result = self
                .send_once_before(
                    HttpRequest {
                        method: "POST".to_owned(),
                        path: path.to_owned(),
                        headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
                        body: body.clone(),
                    },
                    deadline,
                    attempt_context,
                )
                .await;
            if let Some(registration) = registration {
                registration.complete(
                    if result.is_ok() {
                        crate::execution::ExternalOutcome::Confirmed
                    } else {
                        crate::execution::ExternalOutcome::Unknown
                    },
                    vec![],
                )?;
            }
            match result {
                Ok(response) => return Ok(response),
                Err(error) => {
                    if error.code == HostErrorCode::Cancelled {
                        return Err(error);
                    }
                    last_error = Some(error);
                    if attempt + 1 < self.retry.attempts {
                        match operation {
                            Some(operation) => tokio::select! {
                                _ = operation.signal().cancelled() => return Err(cancelled_request()),
                                result = sleep_before_deadline(self.retry.delay, deadline, self.timeout) => result?,
                            },
                            None => {
                                sleep_before_deadline(self.retry.delay, deadline, self.timeout)
                                    .await?
                            }
                        }
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
        let deadline = self.effective_deadline(None);
        self.send_once_before(request, deadline, None).await
    }

    async fn send_once_before(
        &self,
        request: HttpRequest,
        deadline: tokio::time::Instant,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<HttpResponse, HostError> {
        let url = self.authority.join(&request.path)?;
        let method = Method::from_bytes(request.method.as_bytes()).map_err(|error| {
            HostError::new(HostErrorCode::InvalidRequest, "invalid HTTP method")
                .with_detail("error", error.to_string())
        })?;
        let response = self
            .send_request_before(
                method,
                url,
                request.headers,
                request.body.into_bytes(),
                deadline,
                context,
            )
            .await?;
        let body = String::from_utf8(response.body).map_err(|error| {
            HostError::new(
                HostErrorCode::InvalidResponse,
                "HTTP response body is not valid UTF-8",
            )
            .with_detail("error", error.to_string())
        })?;
        Ok(HttpResponse {
            status: response.status,
            headers: response.headers,
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
        let deadline = self.effective_deadline(None);
        let url = self.authority.parse_and_authorize(url.as_ref())?;
        let method = Method::from_bytes(method.as_ref().as_bytes()).map_err(|error| {
            HostError::new(HostErrorCode::InvalidRequest, "invalid HTTP method")
                .with_detail("error", error.to_string())
        })?;
        self.send_request_before(method, url, headers, body, deadline, None)
            .await
    }

    async fn send_request_before(
        &self,
        method: Method,
        url: reqwest::Url,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        deadline: tokio::time::Instant,
        context: Option<&crate::execution::OperationContext>,
    ) -> Result<HttpRawResponse, HostError> {
        if let Some(context) = context {
            context.signal().check()?;
        }
        let operation = async {
            let remaining = remaining_until(deadline, self.timeout)?;
            let connect_timeout = cmp::min(self.timeout.connect_timeout(), remaining);
            let client = resolved_http_client(&self.authority, connect_timeout).await?;
            let mut builder = client.request(method, url).body(body);
            for (name, value) in headers.into_iter().chain(self.auth.headers()) {
                builder = builder.header(
                    parse_header_name(&name)?,
                    parse_header_value(&name, &value)?,
                );
            }
            if let Some(context) = context {
                context.signal().check()?;
            }
            let response = builder
                .send()
                .await
                .map_err(|error| request_error("HTTP request failed", error))?;
            let status = response.status().as_u16();
            let headers = response_headers(response.headers())?;
            let body = response
                .bytes()
                .await
                .map_err(|error| request_error("failed to read HTTP response", error))?;
            Ok(HttpRawResponse {
                status,
                headers,
                body: body.to_vec(),
            })
        };
        // Reqwest's request/body future owns local transport I/O. Dropping it
        // stops the local wait, but does not assert remote rollback or completion.
        tokio::select! {
            _ = async { match context { Some(context) => { let _ = context.signal().cancelled().await; }, None => std::future::pending::<()>().await } } => Err(cancelled_request()),
            result = tokio::time::timeout_at(deadline, operation) => result.map_err(|_| request_deadline_exceeded(self.timeout))?,
        }
    }

    fn effective_deadline(
        &self,
        outer_deadline: Option<tokio::time::Instant>,
    ) -> tokio::time::Instant {
        let configured = tokio::time::Instant::now() + self.timeout.request_deadline();
        outer_deadline.map_or(configured, |outer| cmp::min(configured, outer))
    }
}

fn cancelled_request() -> HostError {
    HostError::new(
        HostErrorCode::Cancelled,
        "HTTP transport wait cancelled; remote completion is unknown",
    )
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

async fn resolved_http_client(
    authority: &TransportEndpointAuthority,
    connect_timeout: Duration,
) -> Result<Client, HostError> {
    let (_, host, _) = authority.endpoint();
    let addresses = TransportEndpointResolver::resolve(authority).await?;
    build_resolved_http_client(host, &addresses, connect_timeout)
}

fn build_resolved_http_client(
    host: &str,
    addresses: &[SocketAddr],
    connect_timeout: Duration,
) -> Result<Client, HostError> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(connect_timeout)
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

fn remaining_until(
    deadline: tokio::time::Instant,
    timeout: TransportTimeoutPolicy,
) -> Result<Duration, HostError> {
    deadline
        .checked_duration_since(tokio::time::Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| request_deadline_exceeded(timeout))
}

async fn sleep_before_deadline(
    delay: Duration,
    deadline: tokio::time::Instant,
    timeout: TransportTimeoutPolicy,
) -> Result<(), HostError> {
    tokio::time::timeout_at(deadline, tokio::time::sleep(delay))
        .await
        .map_err(|_| request_deadline_exceeded(timeout))
}

fn request_error(message: &'static str, error: reqwest::Error) -> HostError {
    let code = if error.is_timeout() {
        HostErrorCode::TimedOut
    } else {
        HostErrorCode::ProviderUnavailable
    };
    HostError::new(code, message).with_detail("error", error.to_string())
}

fn request_deadline_exceeded(timeout: TransportTimeoutPolicy) -> HostError {
    HostError::new(
        HostErrorCode::TimedOut,
        "HTTP transport request deadline exceeded",
    )
    .with_detail(
        "configured_request_deadline_ms",
        timeout.request_deadline().as_millis().to_string(),
    )
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
