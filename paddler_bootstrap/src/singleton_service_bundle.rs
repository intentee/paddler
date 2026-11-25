use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;

use trzcina::ServiceManager;

use crate::agent_bootstrap_config::AgentBootstrapConfig;
use crate::agent_service_bundle::AgentServiceBundle;
use crate::balancer_service_bundle::BalancerServiceBundle;
use crate::bootstrap_error::BootstrapError;
use crate::registers_services::RegistersServices;
use crate::singleton_bootstrap_config::SingletonBootstrapConfig;

fn address_reachable_from_this_host(listening_address: SocketAddr) -> SocketAddr {
    match listening_address.ip() {
        IpAddr::V4(listening_ip) if listening_ip.is_unspecified() => {
            SocketAddr::from((Ipv4Addr::LOCALHOST, listening_address.port()))
        }
        IpAddr::V6(listening_ip) if listening_ip.is_unspecified() => {
            SocketAddr::from((Ipv6Addr::LOCALHOST, listening_address.port()))
        }
        _ => listening_address,
    }
}

pub struct SingletonServiceBundle {
    agent: AgentServiceBundle,
    pub balancer: BalancerServiceBundle,
}

impl SingletonServiceBundle {
    /// # Errors
    ///
    /// Returns the [`BootstrapError`] of [`BalancerServiceBundle::new`] when the balancer services cannot be set up.
    pub async fn new(
        SingletonBootstrapConfig {
            agent_runtime,
            balancer,
        }: SingletonBootstrapConfig,
    ) -> Result<Self, BootstrapError> {
        let balancer = BalancerServiceBundle::new(balancer).await?;
        let agent = AgentServiceBundle::new(AgentBootstrapConfig {
            management_address: address_reachable_from_this_host(balancer.addresses.management),
            runtime: agent_runtime,
        });

        Ok(Self { agent, balancer })
    }
}

impl RegistersServices for SingletonServiceBundle {
    fn register_services(self, service_manager: &mut ServiceManager) {
        self.balancer.register_services(service_manager);
        self.agent.register_services(service_manager);
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::net::Ipv6Addr;
    use std::net::SocketAddr;

    use super::address_reachable_from_this_host;

    #[test]
    fn an_agent_dials_ipv4_loopback_when_its_balancer_listens_on_every_ipv4_interface() {
        assert_eq!(
            address_reachable_from_this_host(SocketAddr::from((Ipv4Addr::UNSPECIFIED, 8060))),
            SocketAddr::from((Ipv4Addr::LOCALHOST, 8060))
        );
    }

    #[test]
    fn an_agent_dials_ipv6_loopback_when_its_balancer_listens_on_every_ipv6_interface() {
        assert_eq!(
            address_reachable_from_this_host(SocketAddr::from((Ipv6Addr::UNSPECIFIED, 8060))),
            SocketAddr::from((Ipv6Addr::LOCALHOST, 8060))
        );
    }

    #[test]
    fn an_agent_dials_the_exact_address_its_balancer_listens_on() {
        let listening_address = SocketAddr::from((Ipv4Addr::new(10, 0, 0, 5), 8060));

        assert_eq!(
            address_reachable_from_this_host(listening_address),
            listening_address
        );
    }
}
