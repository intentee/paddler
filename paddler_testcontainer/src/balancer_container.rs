use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use anyhow::Result;
use anyhow::anyhow;
use paddler_cluster::balancer_addresses::BalancerAddresses;
use paddler_cluster::running_balancer::RunningBalancer;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use url::Host;

use crate::container_managed_process::ContainerManagedProcess;
use crate::image_reference::ImageReference;

const INFERENCE_PORT: u16 = 8061;
const MANAGEMENT_PORT: u16 = 8060;

fn resolve_host(host: Host) -> Result<IpAddr> {
    match host {
        Host::Ipv4(ip) => Ok(IpAddr::V4(ip)),
        Host::Ipv6(ip) => Ok(IpAddr::V6(ip)),
        Host::Domain(domain) if domain == "localhost" => Ok(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        Host::Domain(domain) => Err(anyhow!(
            "docker host {domain:?} is neither an IP address nor localhost; the balancer's mapped ports must be reachable directly from the test process"
        )),
    }
}

pub struct StartedBalancer {
    pub balancer_bridge_ip: IpAddr,
    pub running_balancer: RunningBalancer,
}

impl StartedBalancer {
    pub async fn start(network: &str, image: &ImageReference) -> Result<Self> {
        let container = GenericImage::new(image.name.clone(), image.tag.clone())
            .with_exposed_port(MANAGEMENT_PORT.tcp())
            .with_exposed_port(INFERENCE_PORT.tcp())
            .with_cmd([
                "balancer",
                "--management-addr",
                "0.0.0.0:8060",
                "--inference-addr",
                "0.0.0.0:8061",
            ])
            .with_network(network.to_owned())
            .start()
            .await?;

        let balancer_bridge_ip = container.get_bridge_ip_address().await?;
        let host = resolve_host(container.get_host().await?)?;
        let management_port = container.get_host_port_ipv4(MANAGEMENT_PORT.tcp()).await?;
        let inference_port = container.get_host_port_ipv4(INFERENCE_PORT.tcp()).await?;

        let addresses = BalancerAddresses {
            compat_openai: None,
            inference: SocketAddr::new(host, inference_port),
            management: SocketAddr::new(host, management_port),
        };

        Ok(Self {
            balancer_bridge_ip,
            running_balancer: RunningBalancer::new(
                addresses,
                Box::new(ContainerManagedProcess::new(container)),
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use url::Host;

    use super::resolve_host;

    #[test]
    fn resolves_an_ipv4_host() {
        let resolved = resolve_host(Host::Ipv4(Ipv4Addr::new(192, 168, 0, 5))).unwrap();

        assert_eq!(resolved.to_string(), "192.168.0.5");
    }

    #[test]
    fn resolves_localhost_to_loopback() {
        let resolved = resolve_host(Host::Domain("localhost".to_owned())).unwrap();

        assert!(resolved.is_loopback());
    }

    #[test]
    fn rejects_a_non_local_domain() {
        let error = resolve_host(Host::Domain("docker.example.com".to_owned())).unwrap_err();

        assert!(error.to_string().contains("docker.example.com"));
    }
}
