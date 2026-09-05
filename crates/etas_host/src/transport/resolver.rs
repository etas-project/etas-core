use std::{
    collections::BTreeSet,
    net::{IpAddr, SocketAddr},
};

use crate::network_address::{is_public_address, resembles_noncanonical_ip_literal};
use crate::{HostError, HostErrorCode};

use super::{PrivateResolutionPolicy, TransportEndpointAuthority};

pub(crate) struct TransportEndpointResolver;

impl TransportEndpointResolver {
    pub(crate) async fn resolve(
        authority: &TransportEndpointAuthority,
    ) -> Result<Vec<SocketAddr>, HostError> {
        let (scheme, host, port) = authority.endpoint();
        if host.parse::<IpAddr>().is_err() && resembles_noncanonical_ip_literal(host) {
            return Err(HostError::new(
                HostErrorCode::AuthorityDenied,
                "non-canonical IP address encoding is not permitted",
            )
            .with_detail("scheme", scheme)
            .with_detail("host", host)
            .with_detail("port", port.to_string()));
        }
        let canonical_ip = host.parse::<IpAddr>().ok();
        let addresses = tokio::net::lookup_host((host, port))
            .await
            .map_err(|error| {
                HostError::new(
                    HostErrorCode::ProviderUnavailable,
                    "transport endpoint resolution failed",
                )
                .with_detail("scheme", scheme)
                .with_detail("host", host)
                .with_detail("port", port.to_string())
                .with_detail("error", error.to_string())
            })?
            .collect::<BTreeSet<_>>();
        if addresses.is_empty() {
            return Err(HostError::new(
                HostErrorCode::ProviderUnavailable,
                "transport endpoint resolved to no addresses",
            ));
        }
        for address in &addresses {
            if canonical_ip.is_some_and(|ip| ip != address.ip()) {
                return Err(resolved_address_denied(scheme, host, port, address.ip()));
            }
            if matches!(
                authority.private_resolution(),
                PrivateResolutionPolicy::PublicOnly
            ) && !is_public_address(address.ip())
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
        "transport endpoint resolved to a disallowed address",
    )
    .with_detail("scheme", scheme)
    .with_detail("host", host)
    .with_detail("port", port.to_string())
    .with_detail("address", address.to_string())
}
