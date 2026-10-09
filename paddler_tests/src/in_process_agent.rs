use anyhow::Result;
use async_trait::async_trait;

use paddler_bootstrap::agent_runner::AgentRunner;
use paddler_test_cluster_harness::managed_process::ManagedProcess;
use paddler_test_cluster_harness::process_end::ProcessEnd;

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
    fn terminate(&mut self) -> Result<()> {
        self.runner.cancel();

        Ok(())
    }

    async fn exited(&mut self) -> ProcessEnd {
        self.runner
            .wait_for_completion()
            .await
            .map_or_else(ProcessEnd::failed, |()| ProcessEnd::Clean)
    }
}
