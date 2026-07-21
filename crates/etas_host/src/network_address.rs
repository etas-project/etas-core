use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub(crate) fn resembles_noncanonical_ip_literal(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    if lower.starts_with("0x") || lower.bytes().all(|byte| byte.is_ascii_digit()) {
        return true;
    }
    let mut saw_component = false;
    for component in lower.split('.') {
        if component.is_empty() {
            return false;
        }
        let numeric =
            component.starts_with("0x") || component.bytes().all(|byte| byte.is_ascii_digit());
        if !numeric {
            return false;
        }
        saw_component = true;
    }
    saw_component
}

pub(crate) fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_ipv4(address),
        IpAddr::V6(address) => is_public_ipv6(address),
    }
}

fn is_public_ipv4(address: Ipv4Addr) -> bool {
    let [a, b, ..] = address.octets();
    !address.is_private()
        && !address.is_loopback()
        && !address.is_link_local()
        && !address.is_broadcast()
        && !address.is_documentation()
        && !address.is_unspecified()
        && !address.is_multicast()
        && a != 0
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 198 && (18..=19).contains(&b))
}

fn is_public_ipv6(address: Ipv6Addr) -> bool {
    let segments = address.segments();
    let reserved = address.is_loopback()
        || address.is_unspecified()
        || address.is_multicast()
        || address.is_unique_local()
        || address.is_unicast_link_local()
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
        || (segments[0] & 0xffc0 == 0xfec0);
    !reserved && address.to_ipv4_mapped().is_none_or(is_public_ipv4)
}
