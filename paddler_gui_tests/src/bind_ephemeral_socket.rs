use std::net::SocketAddr;
use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;

pub fn bind_ephemeral_socket() -> Result<SocketAddr> {
    let listener =
        TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral loopback socket")?;

    let local_addr = listener
        .local_addr()
        .context("failed to read local address of bound listener")?;

    drop(listener);

    Ok(local_addr)
}
