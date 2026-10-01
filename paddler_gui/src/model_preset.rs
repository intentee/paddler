use std::fmt;

use paddler_inference_parameters::inference_parameters::InferenceParameters;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

fn huggingface_model(repo_id: &str, filename: &str) -> HuggingFaceModelReference {
    HuggingFaceModelReference {
        repo_id: repo_id.to_owned(),
        filename: filename.to_owned(),
        revision: "main".to_owned(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelPreset {
    Qwen3_0_6B,
    Qwen3_5_0_8B,
}

impl ModelPreset {
    pub const ALL: [Self; 2] = [Self::Qwen3_0_6B, Self::Qwen3_5_0_8B];

    #[must_use]
    pub fn model(self) -> HuggingFaceModelReference {
        match self {
            Self::Qwen3_0_6B => {
                huggingface_model("unsloth/Qwen3-0.6B-GGUF", "Qwen3-0.6B-Q8_0.gguf")
            }
            Self::Qwen3_5_0_8B => {
                huggingface_model("unsloth/Qwen3.5-0.8B-GGUF", "Qwen3.5-0.8B-Q4_K_M.gguf")
            }
        }
    }

    #[must_use]
    pub fn multimodal_projection(self) -> Option<HuggingFaceModelReference> {
        match self {
            Self::Qwen3_0_6B => None,
            Self::Qwen3_5_0_8B => Some(huggingface_model(
                "unsloth/Qwen3.5-0.8B-GGUF",
                "mmproj-F16.gguf",
            )),
        }
    }

    #[must_use]
    pub fn to_balancer_desired_state(self) -> BalancerDesiredState {
        BalancerDesiredState {
            chat_template_override: None,
            inference_parameters: InferenceParameters::default(),
            model: AgentDesiredModel::HuggingFace(self.model()),
            multimodal_projection: self
                .multimodal_projection()
                .map_or(AgentDesiredModel::None, AgentDesiredModel::HuggingFace),
            use_chat_template_override: false,
        }
    }
}

impl fmt::Display for ModelPreset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Qwen3_0_6B => "Qwen 3 0.6B",
            Self::Qwen3_5_0_8B => "Qwen 3.5 0.8B",
        })
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;

    use super::ModelPreset;

    #[test]
    fn a_text_only_preset_runs_without_a_multimodal_projection() {
        let desired_state = ModelPreset::Qwen3_0_6B.to_balancer_desired_state();

        assert_eq!(
            desired_state.model,
            AgentDesiredModel::HuggingFace(HuggingFaceModelReference {
                repo_id: "unsloth/Qwen3-0.6B-GGUF".to_owned(),
                filename: "Qwen3-0.6B-Q8_0.gguf".to_owned(),
                revision: "main".to_owned(),
            })
        );
        assert_eq!(desired_state.multimodal_projection, AgentDesiredModel::None);
    }
}
