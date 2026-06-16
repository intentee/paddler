use anyhow::Result;
use async_trait::async_trait;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use tokio_util::sync::CancellationToken;

use crate::in_process_agent::InProcessAgent;
use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::agent_spawner::AgentSpawner;
use paddler_cluster::managed_process::ManagedProcess;

pub struct InProcessAgentSpawner {
    management_address: String,
}

impl InProcessAgentSpawner {
    #[must_use]
    pub const fn new(management_address: String) -> Self {
        Self { management_address }
    }
}

#[async_trait]
impl AgentSpawner for InProcessAgentSpawner {
    async fn spawn(&self, config: &AgentConfig) -> Result<Box<dyn ManagedProcess>> {
        let runner = AgentRunner::start(AgentRunnerParams {
            agent_name: Some(config.name.clone()),
            cancellation_token: CancellationToken::new(),
            management_address: self.management_address.clone(),
            slots: config.slot_count,
        });

        Ok(Box::new(InProcessAgent::new(runner)))
    }
}
