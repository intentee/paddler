use std::time::Duration;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster_params::ClusterParams;
use crate::longer_than_any_test_run::LONGER_THAN_ANY_TEST_RUN;
use crate::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

pub struct EmbeddingClusterParams {
    pub agents: Vec<AgentConfig>,
    pub buffered_request_timeout: Duration,
    pub inference_parameters: InferenceParameters,
    pub max_buffered_requests: u64,
}

impl EmbeddingClusterParams {
    #[must_use]
    pub fn into_cluster_params(self) -> ClusterParams {
        let Self {
            agents,
            buffered_request_timeout,
            inference_parameters,
            max_buffered_requests,
        } = self;

        ClusterParams {
            agents,
            buffered_request_timeout,
            desired_state: Some(BalancerDesiredState {
                inference_parameters: InferenceParameters {
                    n_gpu_layers: ALL_GPU_LAYERS,
                    ..inference_parameters
                },
                ..nomic_embed_text_v1_5().into_desired_state()
            }),
            max_buffered_requests,
            wait_for_slots_ready: true,
            ..ClusterParams::default()
        }
    }
}

impl Default for EmbeddingClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, 4),
            buffered_request_timeout: LONGER_THAN_ANY_TEST_RUN,
            inference_parameters: InferenceParameters::default(),
            max_buffered_requests: 10,
        }
    }
}
