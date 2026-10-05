use std::net::SocketAddr;
use std::net::TcpListener;

use anyhow::Context as _;
use anyhow::Result;
use url::Url;

pub struct BalancerAddresses {
    pub compat_openai: SocketAddr,
    pub inference: SocketAddr,
    pub management: SocketAddr,
}

impl BalancerAddresses {
    pub fn pick() -> Result<Self> {
        let inference_listener =
            TcpListener::bind("127.0.0.1:0").context("failed to reserve inference service port")?;
        let management_listener = TcpListener::bind("127.0.0.1:0")
            .context("failed to reserve management service port")?;
        let compat_openai_listener = TcpListener::bind("127.0.0.1:0")
            .context("failed to reserve OpenAI-compat service port")?;

        let inference = inference_listener
            .local_addr()
            .context("failed to read inference listener local address")?;
        let management = management_listener
            .local_addr()
            .context("failed to read management listener local address")?;
        let compat_openai = compat_openai_listener
            .local_addr()
            .context("failed to read OpenAI-compat listener local address")?;

        drop((
            inference_listener,
            management_listener,
            compat_openai_listener,
        ));

        Ok(Self {
            compat_openai,
            inference,
            management,
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

#[cfg(test)]
mod tests {
    use super::BalancerAddresses;

    #[test]
    fn pick_reserves_three_distinct_loopback_ports() {
        let addresses = BalancerAddresses::pick().unwrap();

        for address in [
            addresses.inference,
            addresses.management,
            addresses.compat_openai,
        ] {
            assert!(address.ip().is_loopback());
            assert_ne!(address.port(), 0);
        }

        assert_ne!(addresses.inference.port(), addresses.management.port());
        assert_ne!(addresses.inference.port(), addresses.compat_openai.port());
        assert_ne!(addresses.management.port(), addresses.compat_openai.port());
    }

    #[test]
    fn builds_base_urls_for_each_service() {
        let addresses = BalancerAddresses::pick().unwrap();

        assert_eq!(addresses.inference_base_url().unwrap().scheme(), "http");
        assert_eq!(
            addresses.management_base_url().unwrap().port(),
            Some(addresses.management.port())
        );
        assert_eq!(
            addresses.compat_openai_base_url().unwrap().port(),
            Some(addresses.compat_openai.port())
        );
    }
}
