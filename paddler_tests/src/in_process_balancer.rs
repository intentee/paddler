use anyhow::Result;
use async_trait::async_trait;
use paddler_bootstrap::balancer_runner::BalancerRunner;

use crate::managed_process::ManagedProcess;

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
    async fn shutdown(&mut self) -> Result<()> {
        self.runner.cancel();
        self.runner.wait_for_completion().await
    }
}
