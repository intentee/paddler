use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use anyhow::Error;
use anyhow::Result;
use paddler_cluster::balancer_addresses::BalancerAddresses;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::running_balancer::RunningBalancer;
use testcontainers::ContainerAsync;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use url::Host;

use crate::container_managed_process::ContainerManagedProcess;
use crate::error::TestcontainerError;
use crate::image_reference::ImageReference;

const COMPAT_OPENAI_PORT: u16 = 8062;
const INFERENCE_PORT: u16 = 8061;
const MANAGEMENT_PORT: u16 = 8060;

fn resolve_host(host: Host) -> std::result::Result<IpAddr, TestcontainerError> {
    match host {
        Host::Ipv4(ip) => Ok(IpAddr::V4(ip)),
        Host::Ipv6(ip) => Ok(IpAddr::V6(ip)),
        Host::Domain(domain) if domain == "localhost" => Ok(IpAddr::V4(Ipv4Addr::LOCALHOST)),
        Host::Domain(domain) => Err(TestcontainerError::NonLocalDockerHost { domain }),
    }
}

fn balancer_command(service_config: &BalancerServiceConfig) -> Vec<String> {
    let mut command = vec![
        "balancer".to_owned(),
        "--management-addr".to_owned(),
        format!("0.0.0.0:{MANAGEMENT_PORT}"),
        "--inference-addr".to_owned(),
        format!("0.0.0.0:{INFERENCE_PORT}"),
        "--compat-openai-addr".to_owned(),
        format!("0.0.0.0:{COMPAT_OPENAI_PORT}"),
    ];

    command.extend(service_config.command_args());

    command
}

pub struct StartedBalancer {
    pub balancer_bridge_ip: IpAddr,
    pub running_balancer: RunningBalancer,
}

impl StartedBalancer {
    pub async fn start(
        network: &str,
        image: &ImageReference,
        service_config: &BalancerServiceConfig,
    ) -> Result<Self> {
        let container = GenericImage::new(image.name.clone(), image.tag.clone())
            .with_exposed_port(MANAGEMENT_PORT.tcp())
            .with_exposed_port(INFERENCE_PORT.tcp())
            .with_exposed_port(COMPAT_OPENAI_PORT.tcp())
            .with_cmd(balancer_command(service_config))
            .with_network(network.to_owned())
            .start()
            .await?;

        Self::from_container(container).await
    }

    pub async fn from_container(container: ContainerAsync<GenericImage>) -> Result<Self> {
        let balancer_bridge_ip = container.get_bridge_ip_address().await?;

        let mut host_ports = Vec::with_capacity(3);

        for exposed_port in [MANAGEMENT_PORT, INFERENCE_PORT, COMPAT_OPENAI_PORT] {
            host_ports.push(container.get_host_port_ipv4(exposed_port.tcp()).await?);
        }

        let docker_host = container.get_host().await;

        docker_host
            .map_err(Error::from)
            .and_then(|host| resolve_host(host).map_err(Error::from))
            .map(move |host| Self {
                balancer_bridge_ip,
                running_balancer: RunningBalancer::new(
                    BalancerAddresses {
                        compat_openai: SocketAddr::new(host, host_ports[2]),
                        inference: SocketAddr::new(host, host_ports[1]),
                        management: SocketAddr::new(host, host_ports[0]),
                    },
                    Box::new(ContainerManagedProcess::new(container)),
                ),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::net::Ipv6Addr;

    use anyhow::Result;
    use anyhow::anyhow;
    use url::Host;

    use super::resolve_host;
    use crate::error::TestcontainerError;

    #[test]
    fn resolves_an_ipv4_host() {
        let resolved = resolve_host(Host::Ipv4(Ipv4Addr::new(192, 168, 0, 5))).unwrap();

        assert_eq!(resolved.to_string(), "192.168.0.5");
    }

    #[test]
    fn resolves_an_ipv6_host() {
        let resolved =
            resolve_host(Host::Ipv6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1))).unwrap();

        assert_eq!(resolved.to_string(), "fe80::1");
    }

    #[test]
    fn resolves_localhost_to_loopback() {
        let resolved = resolve_host(Host::Domain("localhost".to_owned())).unwrap();

        assert!(resolved.is_loopback());
    }

    #[test]
    fn rejects_a_non_local_domain() -> Result<()> {
        let outcome = resolve_host(Host::Domain("docker.example.com".to_owned()));

        match outcome {
            Err(TestcontainerError::NonLocalDockerHost { domain }) => {
                assert_eq!(domain, "docker.example.com");

                Ok(())
            }
            other => Err(anyhow!("expected NonLocalDockerHost, got {other:?}")),
        }
    }
}
