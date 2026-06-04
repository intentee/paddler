use anyhow::Result;

use crate::agent_config::AgentConfig;
use crate::managed_process::ManagedProcess;

pub trait AgentSpawner: Send + Sync {
    fn spawn(&self, config: &AgentConfig) -> Result<Box<dyn ManagedProcess>>;
}
