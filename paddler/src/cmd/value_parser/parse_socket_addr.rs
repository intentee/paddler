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
                input_addr: input_addr.to_owned(),
                socket_addr: *addr,
            });
        }
    }

    for addr in addrs {
        if addr.is_ipv6() {
            return Ok(ResolvedSocketAddr {
                input_addr: input_addr.to_owned(),
                socket_addr: addr,
            });
        }
    }

    Err(anyhow!("Failed to resolve socket address"))
}

pub fn parse_socket_addr(input_addr: &str) -> Result<ResolvedSocketAddr> {
    match input_addr.parse() {
        Ok(socket_addr) => Ok(ResolvedSocketAddr {
            input_addr: input_addr.to_owned(),
            socket_addr,
        }),
        Err(err) => {
            warn!("Socket addr needs DNS resolution: {err:#?}");

            Ok(resolve_socket_addr(input_addr)?)
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use crate::cmd::value_parser::parse_socket_addr::parse_socket_addr;

    #[test]
    fn test_parses_ip_and_port_directly() -> Result<()> {
        let result = parse_socket_addr("127.0.0.1:8080")?;

        assert_eq!(result.input_addr, "127.0.0.1:8080");
        assert_eq!(result.socket_addr.port(), 8080);
        assert!(result.socket_addr.is_ipv4());

        Ok(())
    }

    #[test]
    fn test_resolves_localhost_via_dns() -> Result<()> {
        let result = parse_socket_addr("localhost:9090")?;

        assert_eq!(result.input_addr, "localhost:9090");
        assert_eq!(result.socket_addr.port(), 9090);

        Ok(())
    }

    #[test]
    fn test_rejects_invalid_address() {
        let result = parse_socket_addr("not-a-valid-host-that-does-not-exist.invalid:1234");

        assert!(result.is_err());
    }

    #[test]
    fn test_parses_ipv6_address() -> Result<()> {
        let result = parse_socket_addr("[::1]:8080")?;

        assert_eq!(result.input_addr, "[::1]:8080");
        assert_eq!(result.socket_addr.port(), 8080);
        assert!(result.socket_addr.is_ipv6());

        Ok(())
    }
}
