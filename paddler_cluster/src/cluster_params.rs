use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;

pub struct ClusterParams {
    pub agents: Vec<AgentConfig>,
    pub desired_state: Option<BalancerDesiredState>,
    pub wait_for_slots_ready: bool,
}

impl Default for ClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 4),
            desired_state: Some(BalancerDesiredState::default()),
            wait_for_slots_ready: true,
        }
    }
}
