use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;

pub struct BalancerAddresses {
    pub compat_openai: String,
    pub inference: String,
    pub management: String,
}

pub fn pick_free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind ephemeral port")?;
    let port = listener
        .local_addr()
        .context("failed to read local address of ephemeral listener")?
        .port();

    drop(listener);

    Ok(port)
}

pub fn pick_balancer_addresses() -> Result<BalancerAddresses> {
    Ok(BalancerAddresses {
        compat_openai: format!("127.0.0.1:{}", pick_free_port()?),
        inference: format!("127.0.0.1:{}", pick_free_port()?),
        management: format!("127.0.0.1:{}", pick_free_port()?),
    })
}
