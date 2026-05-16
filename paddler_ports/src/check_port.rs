use std::io;
use std::net::SocketAddr;
use std::net::TcpListener;

use crate::bound_port::BoundPort;
use crate::port_check_error::PortCheckError;

pub fn check_port(raw: &str) -> Result<BoundPort, PortCheckError> {
    if raw.is_empty() {
        return Err(PortCheckError::Empty);
    }

    let socket_addr: SocketAddr = raw.parse()?;

    match TcpListener::bind(socket_addr) {
        Ok(listener) => Ok(BoundPort {
            socket_addr,
            listener,
        }),
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {
            Err(PortCheckError::InUse { socket_addr })
        }
        Err(error) => Err(PortCheckError::BindFailed {
            socket_addr,
            source: error,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::PortCheckError;
    use super::check_port;

    #[test]
    fn empty_input_reports_empty_error() {
        let result = check_port("");

        assert!(matches!(result, Err(PortCheckError::Empty)));
    }

    #[test]
    fn unparseable_input_reports_unparseable_error() {
        let result = check_port("not-a-socket-addr");

        assert!(matches!(result, Err(PortCheckError::Unparseable { .. })));
    }

    #[test]
    fn input_pointing_at_an_already_bound_port_reports_in_use() -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;

        let result = check_port(&address.to_string());

        assert!(matches!(result, Err(PortCheckError::InUse { .. })));
        Ok(())
    }

    #[test]
    fn input_pointing_at_an_unassigned_address_reports_bind_failed() {
        let result = check_port("192.0.2.1:0");

        assert!(matches!(result, Err(PortCheckError::BindFailed { .. })));
    }

    #[test]
    fn input_pointing_at_a_free_port_returns_a_bound_listener() -> anyhow::Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        drop(listener);

        let bound = check_port(&address.to_string())?;

        if bound.socket_addr.port() != address.port() {
            anyhow::bail!("expected port {} to be preserved", address.port());
        }

        Ok(())
    }
}
