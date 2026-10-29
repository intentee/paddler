use std::num::NonZeroU32;
use std::path::PathBuf;

use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::fixture_path::fixture_path;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;

pub struct DecisionClusterParams {
    pub agents: Vec<AgentConfig>,
    pub context_size: NonZeroU32,
    pub embedding_parameters: EmbeddingParameters,
    pub model_card: ModelCard,
    pub n_batch: BatchSize,
    pub pointer_head: PathBuf,
    pub wait_for_slots_ready: bool,
}

impl DecisionClusterParams {
    pub fn into_cluster_params(self) -> Result<ClusterParams, ModelSourceError> {
        let Self {
            agents,
            context_size,
            embedding_parameters,
            model_card,
            n_batch,
            pointer_head: pointer_head_path,
            wait_for_slots_ready,
        } = self;

        agent_desired_model_at(&pointer_head_path).map(|pointer_head| {
            let desired_state = model_card.into_decision_desired_state(pointer_head);

            ClusterParams {
                agents,
                desired_state: ClusterDesiredState::Apply(Box::new(BalancerDesiredState {
                    embeddings: embedding_parameters,
                    model_runtime_parameters: ModelRuntimeParameters {
                        context_size,
                        n_batch,
                        ..desired_state.model_runtime_parameters
                    },
                    ..desired_state
                })),
                wait_for_slots_ready,
                ..ClusterParams::default()
            }
        })
    }
}

impl Default for DecisionClusterParams {
    fn default() -> Self {
        Self {
            agents: AgentConfig::uniform(1, DECISION_SLOTS_MINIMUM.get()),
            context_size: ModelRuntimeParameters::default().context_size,
            embedding_parameters: EmbeddingParameters::default(),
            model_card: qwen3_5_0_8b(),
            n_batch: BatchSize::default(),
            pointer_head: fixture_path(QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE),
            wait_for_slots_ready: true,
        }
    }
}
