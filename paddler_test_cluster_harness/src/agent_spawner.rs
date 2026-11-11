use anyhow::Result;

use crate::agent_config::AgentConfig;
use crate::managed_process::ManagedProcess;

pub trait AgentSpawner: Send + Sync {
    /// # Errors
    ///
    /// Fails when the agent process cannot be started.
    fn spawn(&self, config: &AgentConfig) -> Result<Box<dyn ManagedProcess>>;
}
