use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

#[must_use]
pub const fn ephemeral_loopback_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}
