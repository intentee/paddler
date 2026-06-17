use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::desired_state_init::DesiredStateInit;

pub struct ClusterParams {
    pub agents: Vec<AgentConfig>,
    pub desired_state: DesiredStateInit,
    pub wait_for_slots_ready: bool,
}

impl Default for ClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 4),
            desired_state: DesiredStateInit::set(BalancerDesiredState::default()),
            wait_for_slots_ready: true,
        }
    }
}
