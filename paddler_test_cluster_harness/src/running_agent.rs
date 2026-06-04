use anyhow::Result;

use crate::agent_config::AgentConfig;
use crate::managed_process::ManagedProcess;

pub struct RunningAgent {
    pub config: AgentConfig,
    process: Box<dyn ManagedProcess>,
}

impl RunningAgent {
    #[must_use]
    pub const fn new(config: AgentConfig, process: Box<dyn ManagedProcess>) -> Self {
        Self { config, process }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.process.shutdown().await
    }
}
