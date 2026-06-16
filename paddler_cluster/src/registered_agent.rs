use anyhow::Result;

use crate::agent_config::AgentConfig;
use crate::managed_process::ManagedProcess;

pub struct RegisteredAgent {
    pub config: AgentConfig,
    pub id: String,
    process: Box<dyn ManagedProcess>,
}

impl RegisteredAgent {
    #[must_use]
    pub const fn new(config: AgentConfig, process: Box<dyn ManagedProcess>, id: String) -> Self {
        Self {
            config,
            id,
            process,
        }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.process.shutdown().await
    }
}
