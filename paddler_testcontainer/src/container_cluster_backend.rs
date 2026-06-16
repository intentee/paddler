use anyhow::Result;
use async_trait::async_trait;
use nanoid::nanoid;
use paddler_cluster::cluster_backend::ClusterBackend;
use paddler_cluster::provisioned_backend::ProvisionedBackend;

use crate::balancer_container::StartedBalancer;
use crate::container_agent_spawner::ContainerAgentSpawner;
use crate::image_reference::ImageReference;

pub struct ContainerClusterBackend;

#[async_trait]
impl ClusterBackend for ContainerClusterBackend {
    async fn provision(&self) -> Result<ProvisionedBackend> {
        let network = format!("paddler-{}", nanoid!());
        let image = ImageReference::resolve()?;

        let StartedBalancer {
            balancer_bridge_ip,
            running_balancer,
        } = StartedBalancer::start(&network, &image).await?;

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
