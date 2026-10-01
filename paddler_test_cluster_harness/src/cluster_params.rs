use std::time::Duration;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::longer_than_any_test_run::LONGER_THAN_ANY_TEST_RUN;

pub struct ClusterParams {
    pub agents: Vec<AgentConfig>,
    pub buffered_request_timeout: Duration,
    pub desired_state: Option<BalancerDesiredState>,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: u64,
    pub state_database_url: String,
    pub wait_for_slots_ready: bool,
}

impl Default for ClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 4),
            buffered_request_timeout: LONGER_THAN_ANY_TEST_RUN,
            desired_state: Some(BalancerDesiredState::default()),
            inference_cors_allowed_hosts: Vec::new(),
            inference_item_timeout: LONGER_THAN_ANY_TEST_RUN,
            management_cors_allowed_hosts: Vec::new(),
            max_buffered_requests: 10,
            state_database_url: "memory://".to_owned(),
            wait_for_slots_ready: true,
        }
    }
}
