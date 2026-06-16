use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;

use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::agent_spawner::AgentSpawner;
use paddler_cluster::managed_process::ManagedProcess;

use crate::spawn_agent_subprocess::spawn_agent_subprocess;
use crate::spawn_agent_subprocess_params::SpawnAgentSubprocessParams;
use crate::subprocess_process::SubprocessProcess;

pub struct SubprocessAgentSpawner {
    binary_path: String,
    management_addr: SocketAddr,
}

impl SubprocessAgentSpawner {
    #[must_use]
    pub const fn new(binary_path: String, management_addr: SocketAddr) -> Self {
        Self {
            binary_path,
            management_addr,
        }
    }
}

#[async_trait]
impl AgentSpawner for SubprocessAgentSpawner {
    async fn spawn(&self, config: &AgentConfig) -> Result<Box<dyn ManagedProcess>> {
        let child = spawn_agent_subprocess(SpawnAgentSubprocessParams {
            binary_path: self.binary_path.clone(),
            management_addr: self.management_addr,
            name: config.name.clone(),
            slots: config.slot_count,
        })?;

        Ok(Box::new(SubprocessProcess::new(child)))
    }
}
