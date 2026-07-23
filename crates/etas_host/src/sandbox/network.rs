use std::{
    collections::BTreeSet,
    net::{IpAddr, SocketAddr, ToSocketAddrs},
};

use crate::network_address::{is_public_address, resembles_noncanonical_ip_literal};
use crate::{HostError, HostErrorCode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkPolicy {
    pub allowed_endpoints: Vec<NetworkEndpoint>,
}

impl NetworkPolicy {
    pub fn deny_all() -> Self {
        Self {
            allowed_endpoints: Vec::new(),
        }
    }

    pub fn allow_endpoints(allowed_endpoints: Vec<NetworkEndpoint>) -> Self {
        Self { allowed_endpoints }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkEndpoint {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

impl NetworkEndpoint {
    pub fn new(scheme: impl Into<String>, host: impl Into<String>, port: u16) -> Self {
        Self {
            scheme: scheme.into(),
            host: host.into(),
            port,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkSandbox {
    policy: NetworkPolicy,
}

impl NetworkSandbox {
    pub fn new(policy: NetworkPolicy) -> Self {
        Self { policy }
    }

    pub fn check_endpoint(&self, scheme: &str, host: &str, port: u16) -> Result<(), HostError> {
        let requested = NetworkEndpoint::new(scheme, host, port);
        if !self.policy.allowed_endpoints.contains(&requested) {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "network endpoint is not allowlisted",
            )
            .with_detail("scheme", scheme)
            .with_detail("host", host)
            .with_detail("port", port.to_string()));
        }
        if host.parse::<IpAddr>().is_err() && resembles_noncanonical_ip_literal(host) {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "non-canonical IP address encoding is not permitted",
            )
            .with_detail("scheme", scheme)
            .with_detail("host", host)
            .with_detail("port", port.to_string()));
        }
        Ok(())
    }

    pub fn resolve_endpoint(
        &self,
        scheme: &str,
        host: &str,
        port: u16,
    ) -> Result<Vec<SocketAddr>, HostError> {
        self.resolve_checked_endpoint(scheme, host, port)
    }

    fn resolve_checked_endpoint(
        &self,
        scheme: &str,
        host: &str,
        port: u16,
    ) -> Result<Vec<SocketAddr>, HostError> {
        self.check_endpoint(scheme, host, port)?;
        let addresses = (host, port)
            .to_socket_addrs()
            .map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "network endpoint resolution failed",
                )
                .with_detail("scheme", scheme)
                .with_detail("host", host)
                .with_detail("port", port.to_string())
                .with_detail("error", error.to_string())
            })?
            .collect::<Vec<_>>();
        self.validate_resolved_addresses(scheme, host, port, addresses)
    }

    pub(crate) fn validate_resolved_addresses(
        &self,
        scheme: &str,
        host: &str,
        port: u16,
        addresses: impl IntoIterator<Item = SocketAddr>,
    ) -> Result<Vec<SocketAddr>, HostError> {
        self.check_endpoint(scheme, host, port)?;
        let canonical_ip = host.parse::<IpAddr>().ok();
        let addresses = addresses.into_iter().collect::<BTreeSet<_>>();
        if addresses.is_empty() {
            return Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "network endpoint resolved to no addresses",
            )
            .with_detail("scheme", scheme)
            .with_detail("host", host)
            .with_detail("port", port.to_string()));
        }

        for address in &addresses {
            if canonical_ip.is_some_and(|ip| ip != address.ip()) {
                return Err(resolved_address_denied(scheme, host, port, address.ip()));
            }
            if canonical_ip.is_none()
                && !is_public_address(address.ip())
                && !self
                    .policy
                    .allowed_endpoints
                    .contains(&NetworkEndpoint::new(
                        scheme,
                        address.ip().to_string(),
                        port,
                    ))
            {
                return Err(resolved_address_denied(scheme, host, port, address.ip()));
            }
        }
        Ok(addresses.into_iter().collect())
    }
}

fn resolved_address_denied(scheme: &str, host: &str, port: u16, address: IpAddr) -> HostError {
    HostError::new(
        HostErrorCode::AuthorityDenied,
        "resolved network address is not allowlisted",
    )
    .with_detail("scheme", scheme)
    .with_detail("host", host)
    .with_detail("port", port.to_string())
    .with_detail("resolved_ip", address.to_string())
}
