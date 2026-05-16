use std::net::SocketAddr;
use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;
use paddler_ports::bind_ephemeral_port::bind_ephemeral_port;
use url::Url;

pub struct BalancerAddresses {
    pub compat_openai: SocketAddr,
    pub compat_openai_listener: Option<TcpListener>,
    pub inference: SocketAddr,
    pub inference_listener: Option<TcpListener>,
    pub management: SocketAddr,
    pub management_listener: Option<TcpListener>,
}

impl BalancerAddresses {
    pub fn pick() -> Result<Self> {
        let inference_bound =
            bind_ephemeral_port().context("failed to reserve inference service port")?;
        let management_bound =
            bind_ephemeral_port().context("failed to reserve management service port")?;
        let compat_openai_bound =
            bind_ephemeral_port().context("failed to reserve OpenAI-compat service port")?;

        Ok(Self {
            compat_openai: compat_openai_bound.socket_addr,
            compat_openai_listener: Some(compat_openai_bound.listener),
            inference: inference_bound.socket_addr,
            inference_listener: Some(inference_bound.listener),
            management: management_bound.socket_addr,
            management_listener: Some(management_bound.listener),
        })
    }

    pub fn compat_openai_base_url(&self) -> Result<Url> {
        Self::base_url_for(self.compat_openai)
    }

    pub fn inference_base_url(&self) -> Result<Url> {
        Self::base_url_for(self.inference)
    }

    pub fn management_base_url(&self) -> Result<Url> {
        Self::base_url_for(self.management)
    }

    fn base_url_for(address: SocketAddr) -> Result<Url> {
        Url::parse(&format!("http://{address}/"))
            .with_context(|| format!("failed to build base URL for {address}"))
    }
}
