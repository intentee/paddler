use paddler_test_cluster_harness::agent_config::AgentConfig;

pub struct PausableAgentParams {
    pub binary_path: String,
    pub config: AgentConfig,
    pub expected_slots_total: u16,
}
