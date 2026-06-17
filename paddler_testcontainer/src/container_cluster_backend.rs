use anyhow::Result;
use async_trait::async_trait;
use nanoid::nanoid;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster_backend::ClusterBackend;
use paddler_cluster::provisioned_backend::ProvisionedBackend;

use crate::balancer_container::StartedBalancer;
use crate::container_agent_spawner::ContainerAgentSpawner;
use crate::image_reference::ImageReference;

#[derive(Default)]
pub struct ContainerClusterBackend {
    service_config: BalancerServiceConfig,
}

impl ContainerClusterBackend {
    #[must_use]
    pub fn with_service_config(self, service_config: BalancerServiceConfig) -> Self {
        Self { service_config }
    }
}

#[async_trait]
impl ClusterBackend for ContainerClusterBackend {
    async fn provision(&self) -> Result<ProvisionedBackend> {
        let network = format!("paddler-{}", nanoid!());
        let image = ImageReference::resolve()?;

        let StartedBalancer {
            balancer_bridge_ip,
            running_balancer,
        } = StartedBalancer::start(&network, &image, &self.service_config).await?;

        Ok(ProvisionedBackend {
            agent_spawner: Box::new(ContainerAgentSpawner {
                balancer_bridge_ip,
                image,
                network,
            }),
            running_balancer,
        })
    }
}
