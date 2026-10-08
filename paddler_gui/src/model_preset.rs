use std::fmt;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::balancer_text_generation_settings::BalancerTextGenerationSettings;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_messaging::multimodal_settings::MultimodalSettings;

fn huggingface_model(repo_id: &str, filename: &str) -> HuggingFaceModelReference {
    HuggingFaceModelReference {
        repo_id: repo_id.to_owned(),
        filename: filename.to_owned(),
        revision: "main".to_owned(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelPreset {
    NomicEmbedTextV1_5,
    Qwen3_0_6B,
    Qwen3_5_0_8B,
}

impl ModelPreset {
    pub const ALL: [Self; 3] = [
        Self::NomicEmbedTextV1_5,
        Self::Qwen3_0_6B,
        Self::Qwen3_5_0_8B,
    ];

    #[must_use]
    pub fn serving(inference_mode: InferenceMode) -> Vec<Self> {
        Self::ALL
            .into_iter()
            .filter(|preset| preset.inference_mode() == inference_mode)
            .collect()
    }

    #[must_use]
    pub const fn inference_mode(self) -> InferenceMode {
        match self {
            Self::NomicEmbedTextV1_5 => InferenceMode::Embeddings,
            Self::Qwen3_0_6B | Self::Qwen3_5_0_8B => InferenceMode::TextGeneration,
        }
    }

    #[must_use]
    pub fn model(self) -> HuggingFaceModelReference {
        match self {
            Self::NomicEmbedTextV1_5 => huggingface_model(
                "nomic-ai/nomic-embed-text-v1.5-GGUF",
                "nomic-embed-text-v1.5.Q8_0.gguf",
            ),
            Self::Qwen3_0_6B => {
                huggingface_model("unsloth/Qwen3-0.6B-GGUF", "Qwen3-0.6B-Q8_0.gguf")
            }
            Self::Qwen3_5_0_8B => {
                huggingface_model("unsloth/Qwen3.5-0.8B-GGUF", "Qwen3.5-0.8B-Q4_K_M.gguf")
            }
        }
    }

    #[must_use]
    pub fn multimodal_projection(self) -> AgentDesiredModel {
        match self {
            Self::NomicEmbedTextV1_5 | Self::Qwen3_0_6B => AgentDesiredModel::None,
            Self::Qwen3_5_0_8B => AgentDesiredModel::HuggingFace(huggingface_model(
                "unsloth/Qwen3.5-0.8B-GGUF",
                "mmproj-F16.gguf",
            )),
        }
    }

    #[must_use]
    pub fn to_balancer_desired_state(self) -> BalancerDesiredState {
        BalancerDesiredState {
            inference_mode: self.inference_mode(),
            model: AgentDesiredModel::HuggingFace(self.model()),
            text_generation: BalancerTextGenerationSettings {
                multimodal: MultimodalSettings {
                    projection: self.multimodal_projection(),
                    ..MultimodalSettings::default()
                },
                ..BalancerTextGenerationSettings::default()
            },
            ..BalancerDesiredState::default()
        }
    }
}

impl fmt::Display for ModelPreset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NomicEmbedTextV1_5 => "Nomic Embed Text v1.5",
            Self::Qwen3_0_6B => "Qwen 3 0.6B",
            Self::Qwen3_5_0_8B => "Qwen 3.5 0.8B",
        })
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::agent_desired_model::AgentDesiredModel;
    use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
    use paddler_messaging::inference_mode::InferenceMode;

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
        assert_eq!(
            desired_state.text_generation.multimodal.projection,
            AgentDesiredModel::None
        );
    }

    #[test]
    fn an_embedding_preset_starts_an_embeddings_cluster() {
        assert_eq!(
            ModelPreset::NomicEmbedTextV1_5
                .to_balancer_desired_state()
                .inference_mode,
            InferenceMode::Embeddings
        );
    }

    #[test]
    fn lists_only_the_presets_serving_the_chosen_mode() {
        assert_eq!(
            ModelPreset::serving(InferenceMode::Embeddings),
            vec![ModelPreset::NomicEmbedTextV1_5]
        );
    }
}
