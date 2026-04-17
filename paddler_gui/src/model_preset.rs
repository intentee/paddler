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
    pub fn available_presets() -> Vec<Self> {
        vec![
            Self {
                display_name: "Qwen 3 0.6B".to_owned(),
                model: HuggingFaceModelReference {
                    repo_id: "unsloth/Qwen3-0.6B-GGUF".to_owned(),
                    filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
                    revision: "main".to_owned(),
                },
                multimodal_projection: None,
                inference_parameters: InferenceParameters::default(),
            },
            Self {
                display_name: "Qwen 3.5 0.8B".to_owned(),
                model: HuggingFaceModelReference {
                    repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_owned(),
                    filename: "Qwen3.5-0.8B-Q4_K_M.gguf".to_owned(),
                    revision: "main".to_owned(),
                },
                multimodal_projection: Some(HuggingFaceModelReference {
                    repo_id: "unsloth/Qwen3.5-0.8B-GGUF".to_owned(),
                    filename: "mmproj-F16.gguf".to_owned(),
                    revision: "main".to_owned(),
                }),
                inference_parameters: InferenceParameters::default(),
            },
        ]
    }

    pub fn to_balancer_desired_state(&self) -> BalancerDesiredState {
        let multimodal_projection = self
            .multimodal_projection
            .as_ref()
            .map_or(AgentDesiredModel::None, |reference| {
                AgentDesiredModel::HuggingFace(reference.clone())
            });

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
