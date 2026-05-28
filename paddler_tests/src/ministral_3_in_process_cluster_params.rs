use crate::agent_config::AgentConfig;

pub struct Ministral3InProcessClusterParams {
    pub agent: AgentConfig,
    pub deterministic_sampling: bool,
}

impl Default for Ministral3InProcessClusterParams {
    fn default() -> Self {
        Self {
            agent: AgentConfig::single(1),
            deterministic_sampling: false,
        }
    }
}
