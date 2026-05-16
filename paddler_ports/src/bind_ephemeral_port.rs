use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;

use crate::bound_port::BoundPort;

pub fn bind_ephemeral_port() -> Result<BoundPort> {
    let listener =
        TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral loopback port")?;

    let socket_addr = listener
        .local_addr()
        .context("failed to read local address of bound listener")?;

    Ok(BoundPort {
        socket_addr,
        listener,
    })
}

#[cfg(test)]
mod tests {
    use super::bind_ephemeral_port;

    #[test]
    fn binding_returns_a_listener_with_a_loopback_address() -> anyhow::Result<()> {
        let bound = bind_ephemeral_port()?;

        assert!(bound.socket_addr.ip().is_loopback());
        assert!(bound.socket_addr.port() > 0);
        Ok(())
    }
}
