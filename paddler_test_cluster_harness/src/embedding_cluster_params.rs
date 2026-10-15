use std::time::Duration;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::agent_config::AgentConfig;
use crate::cluster_desired_state::ClusterDesiredState;
use crate::cluster_params::ClusterParams;
use crate::longer_than_any_test_run::LONGER_THAN_ANY_TEST_RUN;
use crate::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;

pub struct EmbeddingClusterParams {
    pub agents: Vec<AgentConfig>,
    pub buffered_request_timeout: Duration,
    pub embedding_parameters: EmbeddingParameters,
    pub max_buffered_requests: u64,
    pub model_runtime_parameters: ModelRuntimeParameters,
}

impl EmbeddingClusterParams {
    #[must_use]
    pub fn into_cluster_params(self) -> ClusterParams {
        let Self {
            agents,
            buffered_request_timeout,
            embedding_parameters,
            max_buffered_requests,
            model_runtime_parameters,
        } = self;

        ClusterParams {
            agents,
            buffered_request_timeout,
            desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                model_runtime_parameters: ModelRuntimeParameters {
                    n_gpu_layers: ALL_GPU_LAYERS,
                    ..model_runtime_parameters
                },
                ..nomic_embed_text_v1_5().into_embeddings_desired_state(embedding_parameters)
            })),
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
            embedding_parameters: EmbeddingParameters::default(),
            max_buffered_requests: 10,
            model_runtime_parameters: ModelRuntimeParameters::default(),
        }
    }
}
