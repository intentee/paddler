use std::time::Duration;

use paddler_types::balancer_desired_state::BalancerDesiredState;

pub struct InProcessClusterParams {
    pub agent_name: String,
    pub buffered_request_timeout: Duration,
    pub desired_state: BalancerDesiredState,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: i32,
    pub slots_per_agent: i32,
    pub spawn_agent: bool,
    pub wait_for_slots_ready: bool,
}

impl Default for InProcessClusterParams {
    fn default() -> Self {
        Self {
            agent_name: "test-agent".to_owned(),
            buffered_request_timeout: Duration::from_secs(10),
            desired_state: BalancerDesiredState::default(),
            inference_cors_allowed_hosts: Vec::new(),
            inference_item_timeout: Duration::from_secs(30),
            management_cors_allowed_hosts: Vec::new(),
            max_buffered_requests: 10,
            slots_per_agent: 4,
            spawn_agent: true,
            wait_for_slots_ready: true,
        }
    }
}
