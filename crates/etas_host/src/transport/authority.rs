use reqwest::Url;

use crate::{HostError, HostErrorCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrivateResolutionPolicy {
    PublicOnly,
    AllowPrivate,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransportEndpointAuthority {
    base_url: Url,
    scheme: String,
    host: String,
    port: u16,
    private_resolution: PrivateResolutionPolicy,
}

impl TransportEndpointAuthority {
    pub(crate) fn try_new(
        base_url: impl AsRef<str>,
        private_resolution: PrivateResolutionPolicy,
    ) -> Result<Self, HostError> {
        let raw = base_url.as_ref();
        let mut base_url = Url::parse(raw).map_err(|error| {
            HostError::new(HostErrorCode::InvalidRequest, "invalid HTTP base URL")
                .with_detail("url", raw)
                .with_detail("error", error.to_string())
        })?;
        if !matches!(base_url.scheme(), "http" | "https") {
            return Err(HostError::new(
                HostErrorCode::InvalidRequest,
                "HTTP transport endpoint must use http or https",
            )
            .with_detail("scheme", base_url.scheme()));
        }
        let host = base_url
            .host_str()
            .ok_or_else(|| {
                HostError::new(
                    HostErrorCode::InvalidRequest,
                    "HTTP base URL is missing a host",
                )
            })?
            .to_ascii_lowercase();
        let port = base_url.port_or_known_default().ok_or_else(|| {
            HostError::new(
                HostErrorCode::InvalidRequest,
                "HTTP base URL is missing a port",
            )
        })?;
        base_url.set_fragment(None);
        base_url.set_query(None);
        if !base_url.path().ends_with('/') {
            let path = format!("{}/", base_url.path());
            base_url.set_path(&path);
        }
        Ok(Self {
            scheme: base_url.scheme().to_owned(),
            base_url,
            host,
            port,
            private_resolution,
        })
    }

    pub(crate) fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    pub(crate) fn private_resolution(&self) -> PrivateResolutionPolicy {
        self.private_resolution
    }

    pub(crate) fn endpoint(&self) -> (&str, &str, u16) {
        (&self.scheme, &self.host, self.port)
    }

    pub(crate) fn join(&self, path: &str) -> Result<Url, HostError> {
        let joined = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|error| {
                HostError::new(HostErrorCode::InvalidRequest, "invalid HTTP endpoint URL")
                    .with_detail("path", path)
                    .with_detail("error", error.to_string())
            })?;
        self.authorize(&joined)?;
        Ok(joined)
    }

    pub(crate) fn parse_and_authorize(&self, url: &str) -> Result<Url, HostError> {
        let url = Url::parse(url).map_err(|error| {
            HostError::new(HostErrorCode::InvalidRequest, "invalid HTTP endpoint URL")
                .with_detail("url", url)
                .with_detail("error", error.to_string())
        })?;
        self.authorize(&url)?;
        Ok(url)
    }

    fn authorize(&self, url: &Url) -> Result<(), HostError> {
        let host = url.host_str().map(str::to_ascii_lowercase);
        if url.scheme() != self.scheme
            || host.as_deref() != Some(self.host.as_str())
            || url.port_or_known_default() != Some(self.port)
        {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "HTTP request is outside the configured transport endpoint",
            )
            .with_detail("url", url.as_str())
            .with_detail("authority", self.base_url.as_str()));
        }
        Ok(())
    }
}
