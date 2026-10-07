pub mod deepseek_r1_distill_llama_8b;
pub mod every_model_card;
pub mod gemma_3_4b_it;
pub mod gemma_3_4b_it_mmproj;
pub mod gemma_4_e2b_it;
pub mod gemma_4_e2b_it_mmproj;
pub mod ministral_3_3b_reasoning;
pub mod ministral_3_8b_reasoning;
pub mod ministral_3_8b_reasoning_mmproj;
pub mod nomic_embed_text_v1_5;
pub mod qwen2_5_vl_3b;
pub mod qwen2_5_vl_3b_mmproj;
pub mod qwen3_0_6b;
pub mod qwen3_5_0_8b;
pub mod qwen3_5_0_8b_mmproj;
pub mod smolvlm2_256m;
pub mod smolvlm2_256m_mmproj;

use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_inference_settings::BalancerInferenceSettings;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::decision_settings::DecisionSettings;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_messaging::multimodal_settings::MultimodalSettings;

pub struct ModelCard {
    pub reference: HuggingFaceModelReference,
}

impl ModelCard {
    #[must_use]
    pub fn into_agent_desired_model(self) -> AgentDesiredModel {
        AgentDesiredModel::HuggingFace(self.reference)
    }

    #[must_use]
    pub fn into_desired_state(self) -> BalancerDesiredState {
        self.into_desired_state_with_inference_settings(BalancerInferenceSettings::TextGeneration(
            BalancerTextGenerationSettings {
                sampling_parameters: SamplingParameters::deterministic(),
                ..BalancerTextGenerationSettings::default()
            },
        ))
    }

    #[must_use]
    pub fn into_desired_state_with_multimodal_projection(
        self,
        multimodal_projection: Self,
    ) -> BalancerDesiredState {
        self.into_desired_state_with_inference_settings(BalancerInferenceSettings::TextGeneration(
            BalancerTextGenerationSettings {
                multimodal: MultimodalSettings {
                    projection: multimodal_projection.into_agent_desired_model(),
                    ..MultimodalSettings::default()
                },
                sampling_parameters: SamplingParameters::deterministic(),
                ..BalancerTextGenerationSettings::default()
            },
        ))
    }

    #[must_use]
    pub fn into_decision_desired_state(
        self,
        pointer_head: AgentDesiredModel,
    ) -> BalancerDesiredState {
        self.into_desired_state_with_inference_settings(BalancerInferenceSettings::Decision(
            DecisionSettings { pointer_head },
        ))
    }

    #[must_use]
    pub fn into_embeddings_desired_state(
        self,
        embedding_parameters: EmbeddingParameters,
    ) -> BalancerDesiredState {
        self.into_desired_state_with_inference_settings(BalancerInferenceSettings::Embeddings(
            embedding_parameters,
        ))
    }

    fn into_desired_state_with_inference_settings(
        self,
        inference_settings: BalancerInferenceSettings,
    ) -> BalancerDesiredState {
        BalancerDesiredState {
            inference_settings,
            model: self.into_agent_desired_model(),
            model_runtime_parameters: ModelRuntimeParameters {
                n_gpu_layers: ALL_GPU_LAYERS,
                ..ModelRuntimeParameters::default()
            },
        }
    }
}
