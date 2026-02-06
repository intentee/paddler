use std::net::SocketAddr;
use std::net::ToSocketAddrs;

use anyhow::Result;
use anyhow::anyhow;
use log::warn;

use crate::resolved_socket_addr::ResolvedSocketAddr;

fn resolve_socket_addr(input_addr: &str) -> Result<ResolvedSocketAddr> {
    let addrs: Vec<SocketAddr> = input_addr.to_socket_addrs()?.collect();

    for addr in &addrs {
        if addr.is_ipv4() {
            return Ok(ResolvedSocketAddr {
                input_addr: input_addr.to_string(),
                socket_addr: *addr,
            });
        }
    }

    for addr in addrs {
        if addr.is_ipv6() {
            return Ok(ResolvedSocketAddr {
                input_addr: input_addr.to_string(),
                socket_addr: addr,
            });
        }
    }

    Err(anyhow!("Failed to resolve socket address"))
}

pub fn parse_socket_addr(input_addr: &str) -> Result<ResolvedSocketAddr> {
    match input_addr.parse() {
        Ok(socket_addr) => Ok(ResolvedSocketAddr {
            input_addr: input_addr.to_string(),
            socket_addr,
        }),
        Err(err) => {
            warn!("Socket addr needs DNS resolution: {err:#?}");

            Ok(resolve_socket_addr(input_addr)?)
        }
    }
}
