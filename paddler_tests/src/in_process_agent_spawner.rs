use anyhow::Result;
use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_bootstrap::agent_runner::AgentRunnerParams;
use tokio_util::sync::CancellationToken;

use crate::agent_config::AgentConfig;
use crate::agent_spawner::AgentSpawner;
use crate::in_process_agent::InProcessAgent;
use crate::managed_process::ManagedProcess;

pub struct InProcessAgentSpawner {
    management_address: String,
}

impl InProcessAgentSpawner {
    #[must_use]
    pub const fn new(management_address: String) -> Self {
        Self { management_address }
    }
}

impl AgentSpawner for InProcessAgentSpawner {
    fn spawn(&self, config: &AgentConfig) -> Result<Box<dyn ManagedProcess>> {
        let runner = AgentRunner::start(AgentRunnerParams {
            agent_name: Some(config.name.clone()),
            cancellation_token: CancellationToken::new(),
            management_address: self.management_address.clone(),
            slots: config.slot_count,
        });

        Ok(Box::new(InProcessAgent::new(runner)))
    }
}
