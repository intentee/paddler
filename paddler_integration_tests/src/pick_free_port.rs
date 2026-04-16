use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;

pub fn pick_free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral port")?;
    let port = listener
        .local_addr()
        .context("failed to read local address of ephemeral listener")?
        .port();

    drop(listener);

    Ok(port)
}
