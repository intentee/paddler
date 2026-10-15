use anyhow::Result;
use async_trait::async_trait;

use paddler_bootstrap::balancer_runner::BalancerRunner;
use paddler_test_cluster_harness::managed_process::ManagedProcess;
use paddler_test_cluster_harness::process_end::ProcessEnd;

pub struct InProcessBalancer {
    runner: BalancerRunner,
}

impl InProcessBalancer {
    #[must_use]
    pub const fn new(runner: BalancerRunner) -> Self {
        Self { runner }
    }
}

#[async_trait]
impl ManagedProcess for InProcessBalancer {
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
