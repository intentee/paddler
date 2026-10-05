use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

pub const EPHEMERAL_LOOPBACK_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
