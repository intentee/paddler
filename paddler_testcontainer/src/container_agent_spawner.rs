use std::net::IpAddr;

use anyhow::Result;
use async_trait::async_trait;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::agent_spawner::AgentSpawner;
use paddler_cluster::managed_process::ManagedProcess;
use testcontainers::GenericImage;
use testcontainers::ImageExt;
use testcontainers::core::Mount;
use testcontainers::runners::AsyncRunner;

use crate::container_managed_process::ContainerManagedProcess;
use crate::host_huggingface_cache::host_huggingface_cache;
use crate::image_reference::ImageReference;

const CONTAINER_HUGGINGFACE_HOME: &str = "/hf_cache";

pub struct ContainerAgentSpawner {
    pub balancer_bridge_ip: IpAddr,
    pub image: ImageReference,
    pub network: String,
}

#[async_trait]
impl AgentSpawner for ContainerAgentSpawner {
    async fn spawn(&self, config: &AgentConfig) -> Result<Box<dyn ManagedProcess>> {
        let container = GenericImage::new(self.image.name.clone(), self.image.tag.clone())
            .with_cmd([
                "agent".to_owned(),
                "--management-addr".to_owned(),
                format!("{}:8060", self.balancer_bridge_ip),
                "--name".to_owned(),
                config.name.clone(),
                "--slots".to_owned(),
                config.slot_count.to_string(),
            ])
            .with_network(self.network.clone())
            .with_mount(Mount::bind_mount(
                host_huggingface_cache(),
                format!("{CONTAINER_HUGGINGFACE_HOME}/hub"),
            ))
            .with_env_var("HF_HOME", CONTAINER_HUGGINGFACE_HOME)
            .start()
            .await?;

        Ok(Box::new(ContainerManagedProcess::new(container)))
    }
}
