use std::time::Duration;

use paddler_cluster::agent_config::AgentConfig;
use paddler_cluster::balancer_service_config::BalancerServiceConfig;
use paddler_cluster::cluster_params::ClusterParams;
use paddler_cluster::desired_state_init::DesiredStateInit;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::model_card::ModelCard;
use crate::qwen3_embedding_0_6b::qwen3_embedding_0_6b;

pub struct Qwen3EmbeddingClusterParams {
    pub agents: Vec<AgentConfig>,
    pub buffered_request_timeout: Duration,
    pub inference_parameters: InferenceParameters,
    pub max_buffered_requests: i32,
}

impl Qwen3EmbeddingClusterParams {
    #[must_use]
    pub fn service_config(&self) -> BalancerServiceConfig {
        BalancerServiceConfig {
            buffered_request_timeout: self.buffered_request_timeout,
            max_buffered_requests: self.max_buffered_requests,
            ..BalancerServiceConfig::default()
        }
    }

    #[must_use]
    pub fn into_cluster_params(self) -> ClusterParams {
        let ModelCard {
            gpu_layer_count,
            reference,
        } = qwen3_embedding_0_6b();

        ClusterParams {
            agents: self.agents,
            desired_state: DesiredStateInit::set(BalancerDesiredState {
                chat_template_override: None,
                inference_parameters: InferenceParameters {
                    n_gpu_layers: gpu_layer_count,
                    ..self.inference_parameters
                },
                model: AgentDesiredModel::HuggingFace(reference),
                multimodal_projection: AgentDesiredModel::None,
                use_chat_template_override: false,
            }),
            wait_for_slots_ready: true,
        }
    }
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
