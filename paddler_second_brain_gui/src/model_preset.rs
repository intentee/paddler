use std::fmt;

use paddler_types::agent_desired_model::AgentDesiredModel;
use paddler_types::balancer_desired_state::BalancerDesiredState;
use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;

#[derive(Clone, Debug, PartialEq)]
pub struct ModelPreset {
    pub display_name: String,
    pub model: HuggingFaceModelReference,
    pub multimodal_projection: Option<HuggingFaceModelReference>,
    pub inference_parameters: InferenceParameters,
}

impl ModelPreset {
    pub fn available_presets() -> Vec<ModelPreset> {
        vec![ModelPreset {
            display_name: "Qwen 3.5 0.8B".to_string(),
            model: HuggingFaceModelReference {
                repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_string(),
                filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_string(),
                revision: "main".to_string(),
            },
            multimodal_projection: Some(HuggingFaceModelReference {
                repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_string(),
                filename: "mmproj-F16.gguf".to_string(),
                revision: "main".to_string(),
            }),
            inference_parameters: InferenceParameters::default(),
        }]
    }

    pub fn to_balancer_desired_state(&self) -> BalancerDesiredState {
        let multimodal_projection = match &self.multimodal_projection {
            Some(reference) => AgentDesiredModel::HuggingFace(reference.clone()),
            None => AgentDesiredModel::None,
        };

        BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: self.inference_parameters.clone(),
            model: AgentDesiredModel::HuggingFace(self.model.clone()),
            multimodal_projection,
            use_chat_template_override: false,
        }
    }
}

impl fmt::Display for ModelPreset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.display_name)
    }
}
