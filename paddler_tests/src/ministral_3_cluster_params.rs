use paddler_test_cluster_harness::agent_config::AgentConfig;

pub struct Ministral3ClusterParams {
    pub agents: Vec<AgentConfig>,
}

impl Default for Ministral3ClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 1),
        }
    }
}
