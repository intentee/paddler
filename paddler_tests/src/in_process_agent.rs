use anyhow::Result;
use async_trait::async_trait;
use paddler_bootstrap::agent_runner::AgentRunner;

use paddler_test_cluster_harness::managed_process::ManagedProcess;

pub struct InProcessAgent {
    runner: AgentRunner,
}

impl InProcessAgent {
    #[must_use]
    pub const fn new(runner: AgentRunner) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl ManagedProcess for InProcessAgent {
    async fn shutdown(&mut self) -> Result<()> {
        self.runner.cancel();
        self.runner.wait_for_completion().await
    }
}
