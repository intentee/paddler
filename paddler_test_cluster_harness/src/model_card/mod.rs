pub mod deepseek_r1_distill_llama_8b;
pub mod every_model_card;
pub mod gemma_3_4b_it;
pub mod gemma_3_4b_it_mmproj;
pub mod gemma_4_e2b_it;
pub mod gemma_4_e2b_it_mmproj;
pub mod lfm2_colbert_350m;
pub mod ministral_3_3b_reasoning;
pub mod ministral_3_8b_reasoning;
pub mod ministral_3_8b_reasoning_mmproj;
pub mod nomic_embed_text_v1_5;
pub mod qwen2_5_vl_3b;
pub mod qwen2_5_vl_3b_mmproj;
pub mod qwen3_0_6b;
pub mod qwen3_5_0_8b;
pub mod qwen3_5_0_8b_mmproj;
pub mod qwen3_8_27b;
pub mod smolvlm2_256m;
pub mod smolvlm2_256m_mmproj;
pub mod ultravox_v0_5_llama_3_2_1b;
pub mod ultravox_v0_5_llama_3_2_1b_mmproj;

use std::path::PathBuf;

use futures::executor::block_on;
use hf_hub::HFResult;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::multimodal_settings::MultimodalSettings;
use paddler_model_source::huggingface_cache_location::HuggingFaceCacheLocation;
use paddler_model_source::huggingface_model_reference::HuggingFaceModelReference;
use paddler_model_source::model_source::ModelSource;

pub struct ModelCard {
    pub reference: HuggingFaceModelReference,
}

impl ModelCard {
    pub fn cached_path(&self) -> HFResult<PathBuf> {
        HuggingFaceCacheLocation::HubDefault
            .client_builder()
            .build()
            .and_then(|client| {
                block_on(
                    self.reference
                        .cached_path(&self.reference.repository(&client)),
                )
            })
    }

    #[must_use]
    pub fn into_agent_desired_model(self) -> AgentDesiredModel {
        ModelSource::HuggingFace(self.reference).into_agent_desired_model()
    }

    #[must_use]
    pub fn into_desired_state(self) -> BalancerDesiredState {
        BalancerDesiredState {
            model: self.into_agent_desired_model(),
            model_runtime_parameters: ModelRuntimeParameters {
                n_gpu_layers: ALL_GPU_LAYERS,
                ..ModelRuntimeParameters::default()
            },
            text_generation: BalancerTextGenerationSettings {
                sampling_parameters: SamplingParameters::deterministic(),
                ..BalancerTextGenerationSettings::default()
            },
            ..BalancerDesiredState::default()
        }
    }

    #[must_use]
    pub fn into_desired_state_with_multimodal_projection(
        self,
        multimodal_projection: Self,
    ) -> BalancerDesiredState {
        let desired_state = self.into_desired_state();

        BalancerDesiredState {
            text_generation: BalancerTextGenerationSettings {
                multimodal: MultimodalSettings {
                    projection: multimodal_projection.into_agent_desired_model(),
                    ..MultimodalSettings::default()
                },
                ..desired_state.text_generation
            },
            ..desired_state
        }
    }

    #[must_use]
    pub fn into_decision_desired_state(
        self,
        pointer_head: AgentDesiredModel,
    ) -> BalancerDesiredState {
        BalancerDesiredState {
            decision: DecisionSettings { pointer_head },
            ..self.into_desired_state()
        }
    }

    #[must_use]
    pub fn into_desired_state_with_embedding_parameters(
        self,
        embedding_parameters: EmbeddingParameters,
    ) -> BalancerDesiredState {
        BalancerDesiredState {
            embeddings: embedding_parameters,
            ..self.into_desired_state()
        }
    }
}
