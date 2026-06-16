use crate::agent_config::AgentConfig;
use crate::managed_process::ManagedProcess;
use crate::registered_agent::RegisteredAgent;

pub struct SpawnedAgent {
    config: AgentConfig,
    process: Box<dyn ManagedProcess>,
}

impl SpawnedAgent {
    #[must_use]
    pub const fn new(config: AgentConfig, process: Box<dyn ManagedProcess>) -> Self {
        Self { config, process }
    }

    #[must_use]
    pub fn register(self, id: String) -> RegisteredAgent {
        RegisteredAgent::new(self.config, self.process, id)
    }
}
