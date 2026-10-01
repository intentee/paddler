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
use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

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
        BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters {
                n_gpu_layers: ALL_GPU_LAYERS,
                ..InferenceParameters::deterministic()
            },
            model: self.into_agent_desired_model(),
            multimodal_projection: AgentDesiredModel::None,
            use_chat_template_override: false,
        }
    }

    #[must_use]
    pub fn into_desired_state_with_multimodal_projection(
        self,
        multimodal_projection: Self,
    ) -> BalancerDesiredState {
        BalancerDesiredState {
            multimodal_projection: multimodal_projection.into_agent_desired_model(),
            ..self.into_desired_state()
        }
    }
}
