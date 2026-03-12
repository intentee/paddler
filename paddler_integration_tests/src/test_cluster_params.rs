use std::time::Duration;

use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::inference_parameters::InferenceParameters;

use crate::AGENT_DESIRED_MODEL;

pub struct TestClusterParams {
    pub desired_state: BalancerDesiredState,
    pub agent_name: String,
    pub agent_slots: i32,
    pub with_openai: bool,
    pub max_buffered_requests: i32,
    pub buffered_request_timeout: Duration,
    pub inference_item_timeout: Option<Duration>,
    pub wait_for_slots: bool,
}

impl Default for TestClusterParams {
    fn default() -> Self {
        Self {
            desired_state: BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters::default(),
                model: AGENT_DESIRED_MODEL.clone(),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            },
            agent_name: "test-agent".to_string(),
            agent_slots: 4,
            with_openai: false,
            max_buffered_requests: 10,
            buffered_request_timeout: Duration::from_secs(10),
            inference_item_timeout: None,
            wait_for_slots: true,
        }
    }
}
