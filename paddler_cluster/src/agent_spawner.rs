use anyhow::Result;
use async_trait::async_trait;

use crate::agent_config::AgentConfig;
use crate::managed_process::ManagedProcess;

#[async_trait]
pub trait AgentSpawner: Send + Sync {
    async fn spawn(&self, config: &AgentConfig) -> Result<Box<dyn ManagedProcess>>;
}
