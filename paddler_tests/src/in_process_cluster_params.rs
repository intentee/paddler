use std::time::Duration;

use paddler::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;

pub struct InProcessClusterParams {
    pub agent: Option<AgentConfig>,
    pub buffered_request_timeout: Duration,
    pub desired_state: BalancerDesiredState,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: i32,
    pub wait_for_slots_ready: bool,
}

impl Default for InProcessClusterParams {
    fn default() -> Self {
        Self {
            agent: Some(AgentConfig {
                name: "test-agent".to_owned(),
                slot_count: 4,
            }),
            buffered_request_timeout: Duration::from_secs(10),
            desired_state: BalancerDesiredState::default(),
            inference_cors_allowed_hosts: Vec::new(),
            inference_item_timeout: Duration::from_secs(30),
            management_cors_allowed_hosts: Vec::new(),
            max_buffered_requests: 10,
            wait_for_slots_ready: true,
        }
    }
}
