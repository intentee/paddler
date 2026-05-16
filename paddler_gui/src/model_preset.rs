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
    #[must_use]
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

    #[must_use]
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

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use paddler_types::agent_desired_model::AgentDesiredModel;

    use super::ModelPreset;

    #[test]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]
    fn available_presets_returns_at_least_one_preset_per_supported_model() -> Result<()> {
        let presets = ModelPreset::available_presets();

        assert!(
            presets.len() >= 2,
            "expected at least two presets, got {}",
            presets.len()
        );

        Ok(())
    }

    #[test]
    fn preset_without_multimodal_projection_serializes_projection_as_none() -> Result<()> {
        let preset = ModelPreset::available_presets()
            .into_iter()
            .find(|preset| preset.multimodal_projection.is_none())
            .ok_or_else(|| anyhow::anyhow!("expected a preset without multimodal_projection"))?;

        let desired = preset.to_balancer_desired_state();

        assert!(matches!(
            desired.multimodal_projection,
            AgentDesiredModel::None
        ));

        Ok(())
    }

    #[test]
    fn preset_with_multimodal_projection_serializes_projection_as_huggingface() -> Result<()> {
        let preset = ModelPreset::available_presets()
            .into_iter()
            .find(|preset| preset.multimodal_projection.is_some())
            .ok_or_else(|| anyhow::anyhow!("expected a preset with multimodal_projection"))?;

        let desired = preset.to_balancer_desired_state();

        assert!(matches!(
            desired.multimodal_projection,
            AgentDesiredModel::HuggingFace(_)
        ));

        Ok(())
    }

    #[test]
    fn display_impl_returns_the_preset_display_name() -> Result<()> {
        let preset = ModelPreset::available_presets()
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("expected at least one preset"))?;

        assert_eq!(
            format!("{preset}"),
            preset.display_name,
            "Display impl did not match display_name"
        );

        Ok(())
    }
}
