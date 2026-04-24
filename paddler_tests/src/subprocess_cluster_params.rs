use std::time::Duration;

use paddler_types::balancer_desired_state::BalancerDesiredState;

pub struct SubprocessClusterParams {
    pub agent_count: usize,
    pub agent_name_prefix: String,
    pub buffered_request_timeout: Duration,
    pub desired_state: BalancerDesiredState,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: i32,
    pub slots_per_agent: i32,
    pub state_database_url: String,
    pub wait_for_slots_ready: bool,
}

impl Default for SubprocessClusterParams {
    fn default() -> Self {
        Self {
            agent_count: 1,
            agent_name_prefix: "test-agent".to_owned(),
            buffered_request_timeout: Duration::from_secs(10),
            desired_state: BalancerDesiredState::default(),
            inference_cors_allowed_hosts: Vec::new(),
            inference_item_timeout: Duration::from_secs(30),
            management_cors_allowed_hosts: Vec::new(),
            max_buffered_requests: 10,
            slots_per_agent: 4,
            state_database_url: "memory://".to_owned(),
            wait_for_slots_ready: true,
        }
    }
}
