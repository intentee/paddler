use std::num::NonZeroU32;
use std::path::PathBuf;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::batch_size::BatchSize;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::fixture_path::fixture_path;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;
use paddler_test_cluster_harness::synthetic_pointer_head_fixture::QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE;

use crate::decision_backbone::DecisionBackbone;

pub struct DecisionDesiredState {
    pub backbone: DecisionBackbone,
    pub context_size: NonZeroU32,
    pub embedding_parameters: EmbeddingParameters,
    pub n_batch: BatchSize,
    pub n_gpu_layers: i32,
    pub pointer_head: PathBuf,
}

impl DecisionDesiredState {
    pub fn into_balancer_desired_state(self) -> Result<BalancerDesiredState, ModelSourceError> {
        let Self {
            backbone,
            context_size,
            embedding_parameters,
            n_batch,
            n_gpu_layers,
            pointer_head: pointer_head_path,
        } = self;
        let desired_state = backbone.into_desired_state()?;
        let pointer_head = agent_desired_model_at(&pointer_head_path)?;

        Ok(BalancerDesiredState {
            decision: DecisionSettings { pointer_head },
            embeddings: embedding_parameters,
            model_runtime_parameters: ModelRuntimeParameters {
                context_size,
                n_batch,
                n_gpu_layers,
                ..desired_state.model_runtime_parameters
            },
            ..desired_state
        })
    }
}

impl Default for DecisionDesiredState {
    fn default() -> Self {
        Self {
            backbone: DecisionBackbone::HuggingFace(qwen3_5_0_8b()),
            context_size: ModelRuntimeParameters::default().context_size,
            embedding_parameters: EmbeddingParameters::default(),
            n_batch: BatchSize::default(),
            n_gpu_layers: ALL_GPU_LAYERS,
            pointer_head: fixture_path(QWEN3_5_0_8B_SYNTHETIC_POINTER_HEAD_FIXTURE),
        }
    }
}
