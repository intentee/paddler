use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_agent_runner::agent_runner::AgentRunner;
use paddler_agent_runner::agent_runner_config::AgentRunnerConfig;
use paddler_agent_runner::agent_runner_params::AgentRunnerParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::agent_spawner::AgentSpawner;
use paddler_test_cluster_harness::managed_process::ManagedProcess;
use paddler_test_cluster_harness::unexpiring_shutdown_options::unexpiring_shutdown_options;

use crate::in_process_agent::InProcessAgent;

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
            runner_config: AgentRunnerConfig {
                agent_name: Some(config.name.clone()),
                management_address: self.management_address.clone(),
                slots: config.slot_count,
            },
            cancellation_token: CancellationToken::new(),
            shutdown_options: unexpiring_shutdown_options(),
        });

        Ok(Box::new(InProcessAgent::new(runner)))
    }
}
