use std::time::Duration;

use paddler_messaging::inference_parameters::InferenceParameters;

use crate::agent_config::AgentConfig;

pub struct Qwen3EmbeddingClusterParams {
    pub agents: Vec<AgentConfig>,
    pub buffered_request_timeout: Duration,
    pub inference_parameters: InferenceParameters,
    pub max_buffered_requests: i32,
}

impl Default for Qwen3EmbeddingClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 4),
            buffered_request_timeout: Duration::from_secs(10),
            inference_parameters: InferenceParameters::default(),
            max_buffered_requests: 10,
        }
    }
}
