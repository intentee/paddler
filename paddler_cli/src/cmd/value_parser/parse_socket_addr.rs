use anyhow::Result;
use command_handler::value_parser::parse_socket_addr::parse_socket_addr as resolve_socket_addr;
use paddler_balancer::resolved_socket_addr::ResolvedSocketAddr;

pub fn parse_socket_addr(input_addr: &str) -> Result<ResolvedSocketAddr> {
    Ok(ResolvedSocketAddr {
        input_addr: input_addr.to_owned(),
        socket_addr: resolve_socket_addr(input_addr)?,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_socket_addr;

    #[test]
    fn wraps_resolved_address_with_original_input() {
        let result = parse_socket_addr("127.0.0.1:8080").unwrap();

        assert_eq!(result.input_addr, "127.0.0.1:8080");
        assert_eq!(result.socket_addr.port(), 8080);
        assert!(result.socket_addr.is_ipv4());
    }

    #[test]
    fn rejects_invalid_address() {
        let result = parse_socket_addr("not-a-valid-host-that-does-not-exist.invalid:1234");

        assert!(result.is_err());
    }
}
